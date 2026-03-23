#!/usr/bin/env ruby
# frozen_string_literal: true

# mv_tool.rb -- Move (rename) files
# ====================================
#
# === What This Program Does ===
#
# This is a reimplementation of the GNU `mv` utility. It moves files
# and directories from one location to another. When source and
# destination are on the same filesystem, this is a simple rename
# operation (nearly instantaneous). When they're on different
# filesystems, `mv` copies the data and removes the original.
#
# === How mv Works ===
#
#     $ mv old.txt new.txt           # rename a file
#     $ mv file1 file2 dir/          # move files into a directory
#     $ mv olddir/ newdir/           # rename a directory
#
# === Move vs Copy ===
#
# A key difference between `mv` and `cp`:
#   - `mv` removes the source after moving (it's a "cut and paste")
#   - `cp` leaves the source in place (it's a "copy and paste")
#
# On the same filesystem, `mv` is O(1) -- it just updates the directory
# entry. Across filesystems, it's O(n) like `cp` because the data must
# be physically copied.
#
# === Overwrite Modes ===
#
#   -f (force)       Never prompt before overwriting
#   -i (interactive) Always prompt before overwriting
#   -n (no-clobber)  Never overwrite an existing file
#   -u (update)      Only move when source is newer
#   -v (verbose)     Print what is being done
#
# === Implementation ===
#
# We use Ruby's FileUtils.mv, which handles same-filesystem renames
# and cross-filesystem moves transparently.

require "fileutils"
require "coding_adventures_cli_builder"

MV_SPEC_FILE = File.join(File.dirname(__FILE__), "mv.json")

# ---------------------------------------------------------------------------
# Business Logic: mv_move
# ---------------------------------------------------------------------------
# Move +src+ to +dst+ with the given options.
#
# Options hash keys:
#   :force       - Never prompt before overwriting
#   :no_clobber  - Never overwrite existing files
#   :update      - Only move if source is newer than destination
#   :verbose     - Print what is being done
#   :no_target_directory - Treat dst as a regular file, not a directory
#
# Returns a message string if verbose, nil otherwise.
# Raises a string on failure.

def mv_move(src, dst, opts = {})
  # --- Validate source exists -------------------------------------------------
  unless File.exist?(src) || File.symlink?(src)
    raise "mv: cannot stat '#{src}': No such file or directory"
  end

  # --- Resolve destination path -----------------------------------------------
  # If dst is an existing directory (and -T not set), move src inside it.
  actual_dst = if File.directory?(dst) && !opts[:no_target_directory]
    File.join(dst, File.basename(src))
  else
    dst
  end

  # --- No-clobber check -------------------------------------------------------
  if opts[:no_clobber] && File.exist?(actual_dst)
    return nil
  end

  # --- Update check -----------------------------------------------------------
  # Only move when source is newer than destination.
  if opts[:update] && File.exist?(actual_dst)
    return nil if File.mtime(src) <= File.mtime(actual_dst)
  end

  # --- Perform the move -------------------------------------------------------
  begin
    FileUtils.mv(src, actual_dst, force: opts[:force] || false)
  rescue Errno::EACCES
    raise "mv: cannot move '#{src}' to '#{actual_dst}': Permission denied"
  rescue SystemCallError => e
    raise "mv: #{e.message}"
  end

  opts[:verbose] ? "renamed '#{src}' -> '#{actual_dst}'" : nil
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def mv_main
  begin
    result = CodingAdventures::CliBuilder::Parser.new(MV_SPEC_FILE, ["mv"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "mv: #{err.message}" }
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
    sources_raw = result.arguments.fetch("sources", [])
    sources_raw = [sources_raw] if sources_raw.is_a?(String)

    if sources_raw.length < 2
      warn "mv: missing destination file operand"
      exit 1
    end

    dst = sources_raw.last
    sources = sources_raw[0..-2]

    opts = {
      force: result.flags["force"] || false,
      interactive: result.flags["interactive"] || false,
      no_clobber: result.flags["no_clobber"] || false,
      update: result.flags["update"] || false,
      verbose: result.flags["verbose"] || false,
      no_target_directory: result.flags["no_target_directory"] || false,
    }

    exit_code = 0
    sources.each do |src|
      begin
        msg = mv_move(src, dst, opts)
        puts msg if msg
      rescue => e
        warn e.message
        exit_code = 1
      end
    end

    exit exit_code
  end
end

mv_main if __FILE__ == $PROGRAM_NAME
