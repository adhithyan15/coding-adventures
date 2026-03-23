#!/usr/bin/env ruby
# frozen_string_literal: true

# cp_tool.rb -- Copy files and directories
# ==========================================
#
# === What This Program Does ===
#
# This is a reimplementation of the GNU `cp` utility. It copies files
# and directories from one location to another. With the `-R` flag,
# it can copy entire directory trees recursively.
#
# === How cp Works ===
#
#     $ cp source.txt dest.txt       # copy a single file
#     $ cp file1 file2 dir/          # copy multiple files into a directory
#     $ cp -R srcdir/ dstdir/        # copy an entire directory tree
#
# === Copy Semantics ===
#
# When copying a file to a destination:
#   - If the destination is a directory, the file is placed inside it.
#   - If the destination is a file, it is overwritten (unless -n/-i).
#   - If the destination doesn't exist, it is created.
#
# When copying multiple sources, the last argument must be a directory.
#
# === Overwrite Modes ===
#
#   -f (force)       Remove the destination if it can't be opened, then retry.
#   -i (interactive) Prompt before every overwrite.
#   -n (no-clobber)  Never overwrite an existing file.
#   -u (update)      Only copy when source is newer than destination.
#
# These are mutually exclusive: the last one specified wins.
#
# === Implementation ===
#
# We delegate the actual filesystem work to Ruby's `FileUtils` module,
# which provides `cp` and `cp_r` methods that mirror the Unix commands.
# Our business logic handles flag interpretation, conflict resolution,
# and verbose output.

require "fileutils"
require "coding_adventures_cli_builder"

CP_SPEC_FILE = File.join(File.dirname(__FILE__), "cp.json")

# ---------------------------------------------------------------------------
# Business Logic: cp_copy_file
# ---------------------------------------------------------------------------
# Copy a single file from +src+ to +dst+.
#
# Options hash keys:
#   :force       - Remove destination if it can't be opened, then retry
#   :interactive - Prompt before overwrite (not implemented in tests,
#                  requires stdin interaction)
#   :no_clobber  - Never overwrite existing files
#   :update      - Only copy if source is newer than destination
#   :verbose     - Print what is being done
#   :preserve    - Preserve file attributes (mode, timestamps)
#   :link        - Create hard links instead of copying
#   :symbolic_link - Create symbolic links instead of copying
#   :dereference - Follow symlinks in source
#
# Returns a string message if verbose, nil otherwise.
# Raises an error string on failure.

def cp_copy_file(src, dst, opts = {})
  # --- Validate source exists -------------------------------------------------
  unless File.exist?(src) || File.symlink?(src)
    raise "cp: cannot stat '#{src}': No such file or directory"
  end

  # --- If source is a directory, require -R -----------------------------------
  if File.directory?(src) && !File.symlink?(src) && !opts[:recursive]
    raise "cp: -R not specified; omitting directory '#{src}'"
  end

  # --- Resolve destination path -----------------------------------------------
  # If dst is an existing directory, place the file inside it.
  actual_dst = if File.directory?(dst) && !opts[:no_target_directory]
    File.join(dst, File.basename(src))
  else
    dst
  end

  # --- No-clobber check -------------------------------------------------------
  # If -n is set, skip if destination exists.
  if opts[:no_clobber] && File.exist?(actual_dst)
    return nil
  end

  # --- Update check -----------------------------------------------------------
  # If -u is set, only copy when source is newer than destination.
  if opts[:update] && File.exist?(actual_dst)
    return nil if File.mtime(src) <= File.mtime(actual_dst)
  end

  # --- Create hard link instead of copying ------------------------------------
  if opts[:link]
    File.link(src, actual_dst)
    return opts[:verbose] ? "'#{src}' -> '#{actual_dst}'" : nil
  end

  # --- Create symbolic link instead of copying --------------------------------
  if opts[:symbolic_link]
    File.symlink(File.expand_path(src), actual_dst)
    return opts[:verbose] ? "'#{src}' -> '#{actual_dst}'" : nil
  end

  # --- Perform the copy -------------------------------------------------------
  begin
    FileUtils.cp(src, actual_dst, preserve: opts[:preserve] || false)
  rescue Errno::EACCES
    if opts[:force]
      # Force mode: remove the destination and retry.
      FileUtils.rm_f(actual_dst)
      FileUtils.cp(src, actual_dst, preserve: opts[:preserve] || false)
    else
      raise "cp: cannot create regular file '#{actual_dst}': Permission denied"
    end
  end

  opts[:verbose] ? "'#{src}' -> '#{actual_dst}'" : nil
end

# ---------------------------------------------------------------------------
# Business Logic: cp_copy_directory
# ---------------------------------------------------------------------------
# Recursively copy a directory tree from +src+ to +dst+.
#
# This wraps FileUtils.cp_r, which handles the recursive traversal.
# The same overwrite options apply as for cp_copy_file.

def cp_copy_directory(src, dst, opts = {})
  unless File.exist?(src) || File.symlink?(src)
    raise "cp: cannot stat '#{src}': No such file or directory"
  end

  # Resolve destination: if dst is an existing directory, copy inside it.
  actual_dst = if File.directory?(dst) && !opts[:no_target_directory]
    File.join(dst, File.basename(src))
  else
    dst
  end

  # No-clobber: skip if destination already exists.
  if opts[:no_clobber] && File.exist?(actual_dst)
    return nil
  end

  FileUtils.cp_r(src, actual_dst, preserve: opts[:preserve] || false)

  opts[:verbose] ? "'#{src}' -> '#{actual_dst}'" : nil
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def cp_main
  begin
    result = CodingAdventures::CliBuilder::Parser.new(CP_SPEC_FILE, ["cp"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "cp: #{err.message}" }
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

    # The last argument is the destination; the rest are sources.
    if sources_raw.length < 2
      warn "cp: missing destination file operand"
      exit 1
    end

    dst = sources_raw.last
    sources = sources_raw[0..-2]

    opts = {
      force: result.flags["force"] || false,
      interactive: result.flags["interactive"] || false,
      no_clobber: result.flags["no_clobber"] || false,
      recursive: result.flags["recursive"] || result.flags["archive"] || false,
      update: result.flags["update"] || false,
      verbose: result.flags["verbose"] || false,
      preserve: result.flags["archive"] || !result.flags.fetch("preserve", "").to_s.empty?,
      link: result.flags["link"] || false,
      symbolic_link: result.flags["symbolic_link"] || false,
      dereference: result.flags["dereference"] || false,
      no_target_directory: result.flags["no_target_directory"] || false,
    }

    exit_code = 0
    sources.each do |src|
      begin
        msg = if File.directory?(src) && opts[:recursive]
          cp_copy_directory(src, dst, opts)
        else
          cp_copy_file(src, dst, opts)
        end
        puts msg if msg
      rescue => e
        warn e.message
        exit_code = 1
      end
    end

    exit exit_code
  end
end

cp_main if __FILE__ == $PROGRAM_NAME
