#!/usr/bin/env ruby
# frozen_string_literal: true

# df_tool.rb -- Report file system disk space usage
# ====================================================
#
# === What This Program Does ===
#
# This is a reimplementation of the GNU `df` utility. It displays
# information about file system disk space usage: how much space is
# available, how much is used, and where the file system is mounted.
#
# === Default Output ===
#
#     $ df
#     Filesystem     1K-blocks      Used Available Use% Mounted on
#     /dev/disk1     976490576 789012345 187478231  81% /
#
# === How We Get Filesystem Information ===
#
# There is no portable pure-Ruby way to query filesystem stats (the
# statfs/statvfs system calls aren't wrapped by Ruby's standard library).
# We shell out to the system `df` command and parse its output.
#
# This is the same approach used by many Ruby system administration
# tools. The output format is normalized using the POSIX (-P) flag
# to ensure consistent, parseable output across platforms.
#
# === Human-Readable Mode (-h) ===
#
# With -h, sizes are displayed with K/M/G/T suffixes using powers
# of 1024. With -H (--si), powers of 1000 are used instead.
#
# === Flags ===
#
#   -a    Include pseudo/inaccessible filesystems
#   -h    Human-readable sizes (powers of 1024)
#   -H    SI sizes (powers of 1000)
#   -i    Show inode information instead of block usage
#   -l    Show only local filesystems
#   -P    POSIX output format
#   -T    Show filesystem type
#   -t    Limit to filesystem type
#   -x    Exclude filesystem type
#   --total  Show grand total

require "coding_adventures_cli_builder"

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

DF_SPEC_FILE = File.join(File.dirname(__FILE__), "df.json")

# ---------------------------------------------------------------------------
# Data class: FilesystemInfo
# ---------------------------------------------------------------------------
# Holds parsed information about one filesystem mount point.

FilesystemInfo = Struct.new(
  :filesystem, :fstype, :blocks, :used, :available, :use_percent, :mounted_on,
  keyword_init: true
)

# ---------------------------------------------------------------------------
# Business Logic: get_filesystem_info
# ---------------------------------------------------------------------------
# Query the system for filesystem information by running the `df` command
# and parsing its output.
#
# We use `df -P` for POSIX-standard output format, which guarantees:
#   - One line per filesystem (no wrapping)
#   - Consistent column layout
#   - 512-byte blocks by default (or 1K with -k)
#
# Parameters:
#   paths - Array of paths to query (empty = all filesystems)
#   flags - Hash of flag values from CLI Builder
#
# Returns: An array of FilesystemInfo structs.

def get_filesystem_info(paths, flags)
  # Build the df command
  cmd_parts = ["df", "-Pk"]  # POSIX format, 1K blocks

  cmd_parts << "-a" if flags["all"]
  cmd_parts << "-i" if flags["inodes"]
  cmd_parts << "-l" if flags["local"]
  cmd_parts << "-T" if flags["print_type"]
  cmd_parts.concat(["-t", flags["type"]]) if flags["type"]
  cmd_parts.concat(["-x", flags["exclude_type"]]) if flags["exclude_type"]

  cmd_parts.concat(paths) unless paths.empty?

  # Execute df and parse output
  output = run_df_command(cmd_parts)
  parse_df_output(output, flags)
end

# ---------------------------------------------------------------------------
# Business Logic: run_df_command
# ---------------------------------------------------------------------------
# Execute the df command and return its stdout.
#
# We use IO.popen for safety (no shell injection). If the command
# fails, we return an empty string.

def run_df_command(cmd_parts)
  IO.popen(cmd_parts, "r", err: [:child, :out]) do |io|
    io.read
  end
rescue Errno::ENOENT
  warn "df: command not found"
  ""
end

# ---------------------------------------------------------------------------
# Business Logic: parse_df_output
# ---------------------------------------------------------------------------
# Parse the output of `df -Pk` into FilesystemInfo structs.
#
# The POSIX format has these columns:
#   Filesystem  1024-blocks  Used  Available  Capacity  Mounted-on
#
# Parameters:
#   output - String output from df command
#   flags  - CLI flags hash
#
# Returns: Array of FilesystemInfo structs.

def parse_df_output(output, flags)
  lines = output.split("\n")
  return [] if lines.length < 2

  # Skip the header line
  lines[1..].filter_map do |line|
    fields = line.split(/\s+/, 6)
    next if fields.length < 6

    fs = fields[0]
    blocks = fields[1].to_i
    used = fields[2].to_i
    available = fields[3].to_i
    use_pct = fields[4]
    mounted = fields[5]

    # Filter by filesystem type if requested
    # (Note: -Pk doesn't include type column; we skip type filtering
    # when parsing POSIX output since the system df handles it)

    FilesystemInfo.new(
      filesystem: fs,
      fstype: nil,
      blocks: blocks,
      used: used,
      available: available,
      use_percent: use_pct,
      mounted_on: mounted
    )
  end
end

# ---------------------------------------------------------------------------
# Business Logic: format_size
# ---------------------------------------------------------------------------
# Format a size value (in 1K blocks) for display.
#
# In human-readable mode (-h), uses K/M/G/T suffixes with 1024-based
# units. In SI mode (-H), uses 1000-based units.
#
# Parameters:
#   blocks         - Size in 1K blocks
#   human_readable - Whether to use -h format
#   si             - Whether to use -H (SI) format
#
# Returns: Formatted string.

def format_size(blocks, human_readable, si)
  if human_readable || si
    base = si ? 1000.0 : 1024.0
    suffixes = ["K", "M", "G", "T", "P", "E"]
    # blocks are already in 1K units, so start with kilobytes
    value = blocks.to_f

    suffixes.each_with_index do |suffix, idx|
      if value < base || idx == suffixes.length - 1
        if value < 10
          return format("%.1f%s", value, suffix)
        else
          return format("%d%s", value.round, suffix)
        end
      end
      value /= base
    end
  end

  blocks.to_s
end

# ---------------------------------------------------------------------------
# Business Logic: format_df_output
# ---------------------------------------------------------------------------
# Format filesystem info for display, handling human-readable and
# other output modes.
#
# Parameters:
#   entries - Array of FilesystemInfo structs
#   flags   - CLI flags hash
#
# Returns: Array of formatted output lines.

def format_df_output(entries, flags)
  human_readable = flags["human_readable"]
  si = flags["si"]
  show_total = flags["total"]

  lines = []

  # Header
  header = format("%-20s %10s %10s %10s %5s %s",
                   "Filesystem", "1K-blocks", "Used", "Available", "Use%", "Mounted on")
  lines << header

  entries.each do |entry|
    size_str = format_size(entry.blocks, human_readable, si)
    used_str = format_size(entry.used, human_readable, si)
    avail_str = format_size(entry.available, human_readable, si)

    lines << format("%-20s %10s %10s %10s %5s %s",
                    entry.filesystem, size_str, used_str, avail_str,
                    entry.use_percent, entry.mounted_on)
  end

  # Grand total
  if show_total && entries.length > 0
    total_blocks = entries.sum(&:blocks)
    total_used = entries.sum(&:used)
    total_avail = entries.sum(&:available)
    total_pct = if total_blocks > 0
                  "#{(total_used * 100.0 / total_blocks).round}%"
                else
                  "0%"
                end

    size_str = format_size(total_blocks, human_readable, si)
    used_str = format_size(total_used, human_readable, si)
    avail_str = format_size(total_avail, human_readable, si)

    lines << format("%-20s %10s %10s %10s %5s %s",
                    "total", size_str, used_str, avail_str, total_pct, "-")
  end

  lines
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def df_main
  # --- Step 1: Parse arguments ---------------------------------------------
  begin
    result = CodingAdventures::CliBuilder::Parser.new(DF_SPEC_FILE, ["df"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "df: #{err.message}" }
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
    paths = result.arguments.fetch("files", [])
    entries = get_filesystem_info(paths, result.flags)
    lines = format_df_output(entries, result.flags)
    lines.each { |line| puts line }
  end
end

# Only run main when this file is executed directly.
df_main if __FILE__ == $PROGRAM_NAME
