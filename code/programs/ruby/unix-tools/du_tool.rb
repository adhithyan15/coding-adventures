#!/usr/bin/env ruby
# frozen_string_literal: true

# du_tool.rb -- Estimate file space usage
# ==========================================
#
# === What This Program Does ===
#
# This is a reimplementation of the GNU `du` utility. It estimates
# the disk space used by files and directories. By default, it
# recursively walks each directory argument and reports the total
# size for each subdirectory.
#
# === Default Behavior ===
#
#     $ du /tmp
#     4       /tmp/foo
#     8       /tmp/bar
#     16      /tmp
#
# Sizes are reported in 1K blocks by default. Each directory gets
# a line showing its total size (including all files within it).
#
# === How It Works ===
#
# We use Ruby's `Find.find` to walk the directory tree. For each
# file, we query its size with `File.size`. Directory sizes are
# accumulated bottom-up: a directory's size includes all its
# contents.
#
# === Key Flags ===
#
#   -a           Show sizes for all files, not just directories
#   -h           Human-readable output (K, M, G suffixes)
#   --si         Like -h but use powers of 1000
#   -s           Show only a total for each argument
#   -c           Show a grand total
#   -d N         Limit output depth to N levels
#   -L           Follow symbolic links
#   --exclude    Exclude files matching PATTERN
#   -0           End lines with NUL instead of newline
#
# === Size Calculation ===
#
# We measure apparent file size (File.size), which reports the number
# of bytes in the file. This differs from actual disk usage (which
# depends on block size and filesystem overhead), but is the most
# portable approach in Ruby.
#
# Sizes are reported in 1K blocks (rounding up), matching the default
# behavior of GNU du.

require "find"
require "coding_adventures_cli_builder"

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

DU_SPEC_FILE = File.join(File.dirname(__FILE__), "du.json")

# ---------------------------------------------------------------------------
# Business Logic: format_du_size
# ---------------------------------------------------------------------------
# Format a size in bytes for display.
#
# By default, sizes are in 1K blocks (bytes / 1024, rounded up).
# In human-readable mode, uses K/M/G/T suffixes.
#
# Parameters:
#   bytes          - Size in bytes
#   human_readable - Whether to use -h format
#   si             - Whether to use --si format
#
# Returns: Formatted string.

def format_du_size(bytes, human_readable, si)
  if human_readable || si
    base = si ? 1000.0 : 1024.0
    suffixes = ["B", "K", "M", "G", "T", "P"]
    value = bytes.to_f

    suffixes.each_with_index do |suffix, idx|
      if value < base || idx == suffixes.length - 1
        if suffix == "B"
          return format("%d", value.round)
        elsif value < 10
          return format("%.1f%s", value, suffix)
        else
          return format("%d%s", value.round, suffix)
        end
      end
      value /= base
    end
  end

  # Default: 1K blocks (round up)
  blocks = (bytes + 1023) / 1024
  blocks.to_s
end

# ---------------------------------------------------------------------------
# Business Logic: matches_exclude?
# ---------------------------------------------------------------------------
# Check if a path matches any of the exclude patterns.
#
# Parameters:
#   path     - The file path to check
#   patterns - Array of glob patterns to exclude
#
# Returns: true if the path should be excluded.

def matches_exclude?(path, patterns)
  return false if patterns.nil? || patterns.empty?

  basename = File.basename(path)
  patterns.any? do |pattern|
    File.fnmatch(pattern, basename, File::FNM_PATHNAME) ||
      File.fnmatch(pattern, path, File::FNM_PATHNAME)
  end
end

# ---------------------------------------------------------------------------
# Business Logic: disk_usage
# ---------------------------------------------------------------------------
# Calculate disk usage for a path, returning an array of [size, path]
# entries to display.
#
# The algorithm walks the directory tree depth-first, accumulating
# sizes. Each directory's size includes all files and subdirectories
# within it.
#
# Parameters:
#   path  - The root path to measure
#   flags - Hash of flag values from CLI Builder
#
# Returns: Array of [bytes, path] pairs.

def disk_usage(path, flags)
  show_all = flags["all"]
  summarize = flags["summarize"]
  max_depth = flags["max_depth"]
  follow_links = flags["dereference"]
  exclude_patterns = flags["exclude"]
  exclude_patterns = [exclude_patterns] if exclude_patterns.is_a?(String)

  # Calculate the base depth for relative depth computation
  base_depth = path.count(File::SEPARATOR)
  base_depth -= 1 if path.end_with?(File::SEPARATOR)

  # Walk the tree and accumulate sizes per directory
  dir_sizes = Hash.new(0)
  entries = []

  begin
    Find.find(path) do |found_path|
      # Check exclude patterns
      if matches_exclude?(found_path, exclude_patterns)
        Find.prune if File.directory?(found_path)
        next
      end

      # Handle symbolic links
      if File.symlink?(found_path) && !follow_links
        # Don't follow symlinks; count the link itself
        size = begin
          File.lstat(found_path).size
        rescue StandardError
          0
        end
      else
        size = begin
          File.size(found_path)
        rescue StandardError
          0
        end
      end

      # Accumulate size into this file's directory and all parent dirs
      if File.directory?(found_path)
        dir_sizes[found_path] += size
      else
        # Add file size to its parent directory
        parent = File.dirname(found_path)
        dir_sizes[parent] += size

        # Record file entry if -a is set
        if show_all
          depth = found_path.count(File::SEPARATOR) - base_depth
          entries << [size, found_path, depth]
        end
      end
    end
  rescue Errno::ENOENT
    warn "du: cannot access '#{path}': No such file or directory"
    return []
  rescue Errno::EACCES
    warn "du: cannot read directory '#{path}': Permission denied"
    return []
  end

  # Propagate directory sizes upward
  # Sort directories by depth (deepest first) to propagate bottom-up
  sorted_dirs = dir_sizes.keys.sort_by { |d| -d.count(File::SEPARATOR) }
  sorted_dirs.each do |dir|
    parent = File.dirname(dir)
    if parent != dir && dir_sizes.key?(parent)
      dir_sizes[parent] += dir_sizes[dir]
    end
  end

  # Build output entries for directories
  sorted_dirs.reverse.each do |dir|
    depth = dir.count(File::SEPARATOR) - base_depth
    entries << [dir_sizes[dir], dir, depth]
  end

  # Sort entries by path for consistent output
  entries.sort_by! { |_, p, _| p }

  # Apply filters
  results = []
  entries.each do |size, entry_path, depth|
    next if summarize && entry_path != path
    next if max_depth && depth > max_depth

    results << [size, entry_path]
  end

  results
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def du_main
  # --- Step 1: Parse arguments ---------------------------------------------
  begin
    result = CodingAdventures::CliBuilder::Parser.new(DU_SPEC_FILE, ["du"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "du: #{err.message}" }
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
    paths = result.arguments.fetch("files", ["."])
    human_readable = result.flags["human_readable"]
    si = result.flags["si"]
    show_total = result.flags["total"]
    line_end = result.flags["null"] ? "\0" : "\n"

    grand_total = 0

    paths.each do |path|
      entries = disk_usage(path, result.flags)
      entries.each do |size, entry_path|
        formatted = format_du_size(size, human_readable, si)
        print "#{formatted}\t#{entry_path}#{line_end}"
      end
      # The last entry for this path is the total
      grand_total += entries.last[0] if entries.last
    end

    if show_total
      formatted = format_du_size(grand_total, human_readable, si)
      print "#{formatted}\ttotal#{line_end}"
    end
  end
end

# Only run main when this file is executed directly.
du_main if __FILE__ == $PROGRAM_NAME
