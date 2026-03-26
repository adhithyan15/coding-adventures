#!/usr/bin/env ruby
# frozen_string_literal: true

# ls_tool.rb -- List directory contents
# =======================================
#
# === What This Program Does ===
#
# This is a reimplementation of the GNU `ls` utility. It lists
# information about files and directories. By default, it lists the
# contents of the current directory in alphabetical order.
#
# === How ls Works ===
#
#     $ ls                # list current directory
#     $ ls -la            # long format, show hidden files
#     $ ls -lhS           # long format, human sizes, sort by size
#     $ ls -R             # recursive listing
#
# === Display Modes ===
#
# ls has several display modes:
#   - Default: multi-column (we simplify to one-per-line for piping)
#   - `-l`: long format showing permissions, owner, size, date, name
#   - `-1`: explicitly one file per line
#
# === Sorting ===
#
# By default, entries are sorted alphabetically. Flags change this:
#   -S  Sort by file size (largest first)
#   -t  Sort by modification time (newest first)
#   -X  Sort by file extension
#   -v  Natural version sort
#   -U  Unsorted (directory order)
#   -r  Reverse any sort order
#
# === Hidden Files ===
#
#   -a  Show all entries including . and ..
#   -A  Show almost all (hidden files, but not . and ..)
#
# === Implementation ===
#
# We use Dir.entries to get directory contents and File.stat (or
# File.lstat for symlinks) to retrieve metadata for long listings.
# The formatting logic lives in ls_format_entry so it can be tested
# independently of the directory-reading logic.

require "etc"
require "coding_adventures_cli_builder"

LS_SPEC_FILE = File.join(File.dirname(__FILE__), "ls.json")

# ---------------------------------------------------------------------------
# Business Logic: ls_human_readable_size
# ---------------------------------------------------------------------------
# Convert a byte count to a human-readable string like "1.5K" or "2.3M".
#
# This follows the IEC convention (powers of 1024) when si is false,
# and SI convention (powers of 1000) when si is true.
#
# Example:
#   ls_human_readable_size(1536)        => "1.5K"
#   ls_human_readable_size(1048576)     => "1.0M"
#   ls_human_readable_size(500)         => "500"

def ls_human_readable_size(bytes, si: false)
  base = si ? 1000.0 : 1024.0
  units = si ? %w[B kB MB GB TB PB] : %w[B K M G T P]

  return bytes.to_s if bytes < base

  unit_index = 0
  size = bytes.to_f
  while size >= base && unit_index < units.length - 1
    size /= base
    unit_index += 1
  end

  # Format: if the value is a whole number, show one decimal place.
  formatted = if size == size.to_i.to_f
    format("%.1f", size)
  else
    format("%.1f", size)
  end

  "#{formatted}#{units[unit_index]}"
end

# ---------------------------------------------------------------------------
# Business Logic: ls_format_entry
# ---------------------------------------------------------------------------
# Format a single directory entry for display.
#
# In short mode, returns just the name (with optional -F classifier).
# In long mode (-l), returns a line like:
#   -rw-r--r-- 1 user group 1234 Mar 15 10:30 filename
#
# Options:
#   :long            - Use long listing format
#   :human_readable  - Show sizes in human-readable form
#   :si              - Use powers of 1000 instead of 1024
#   :classify        - Append type indicator (/ for dirs, * for exec, etc.)
#   :inode           - Show inode number
#   :numeric_uid_gid - Show numeric user/group instead of names
#   :no_group        - Don't show group in long listing

def ls_format_entry(path, name, opts = {})
  parts = []

  # --- Inode number (optional) ------------------------------------------------
  if opts[:inode]
    stat = File.lstat(path)
    parts << stat.ino.to_s
  end

  # --- Long format ------------------------------------------------------------
  if opts[:long]
    stat = File.lstat(path)

    # File type + permissions string.
    mode = ls_format_mode(stat)

    # Number of hard links.
    nlinks = stat.nlink.to_s

    # Owner and group names (or numeric IDs).
    if opts[:numeric_uid_gid]
      owner = stat.uid.to_s
      group = stat.gid.to_s
    else
      owner = begin
        Etc.getpwuid(stat.uid).name
      rescue ArgumentError
        stat.uid.to_s
      end
      group = begin
        Etc.getgrgid(stat.gid).name
      rescue ArgumentError
        stat.gid.to_s
      end
    end

    # File size.
    size = if opts[:human_readable]
      ls_human_readable_size(stat.size, si: opts[:si] || false)
    else
      stat.size.to_s
    end

    # Modification time.
    mtime = stat.mtime
    time_str = if (Time.now - mtime).abs > 180 * 24 * 3600
      mtime.strftime("%b %e  %Y")
    else
      mtime.strftime("%b %e %H:%M")
    end

    # Build the long-format line.
    if opts[:no_group]
      parts << "#{mode} #{nlinks} #{owner} #{size} #{time_str}"
    else
      parts << "#{mode} #{nlinks} #{owner} #{group} #{size} #{time_str}"
    end
  end

  # --- Name with optional classifier -----------------------------------------
  display_name = name.dup
  if opts[:classify]
    display_name << ls_classify_char(path)
  end

  parts << display_name
  parts.join(" ")
end

# ---------------------------------------------------------------------------
# Helper: ls_format_mode
# ---------------------------------------------------------------------------
# Convert a File::Stat mode to a permission string like "-rwxr-xr-x".
#
# The first character indicates the file type:
#   d = directory, l = symlink, c = char device, b = block device,
#   p = pipe (FIFO), s = socket, - = regular file
#
# The remaining 9 characters show read/write/execute for owner, group,
# and others respectively.

def ls_format_mode(stat)
  # File type character.
  type_char = case stat.ftype
  when "directory"         then "d"
  when "link"              then "l"
  when "characterSpecial"  then "c"
  when "blockSpecial"      then "b"
  when "fifo"              then "p"
  when "socket"            then "s"
  else                          "-"
  end

  # Permission bits: each group of 3 bits maps to rwx.
  mode = stat.mode
  perms = +""
  [6, 3, 0].each do |shift|
    bits = (mode >> shift) & 0x7
    perms << ((bits & 4) != 0 ? "r" : "-")
    perms << ((bits & 2) != 0 ? "w" : "-")
    perms << ((bits & 1) != 0 ? "x" : "-")
  end

  type_char + perms
end

# ---------------------------------------------------------------------------
# Helper: ls_classify_char
# ---------------------------------------------------------------------------
# Return the -F classifier character for a file:
#   /  directory
#   *  executable
#   @  symlink
#   |  FIFO (named pipe)
#   =  socket
#   (nothing) regular file

def ls_classify_char(path)
  if File.symlink?(path)
    "@"
  elsif File.directory?(path)
    "/"
  elsif File.executable?(path)
    "*"
  elsif File.pipe?(path)
    "|"
  elsif File.socket?(path)
    "="
  else
    ""
  end
end

# ---------------------------------------------------------------------------
# Business Logic: ls_list
# ---------------------------------------------------------------------------
# List the contents of a path and return an array of formatted strings.
#
# If the path is a file, return information about that file.
# If the path is a directory, return its contents (filtered and sorted).
#
# Options:
#   :all             - Include hidden entries (. and ..)
#   :almost_all      - Include hidden entries but not . and ..
#   :long            - Long listing format
#   :recursive       - List subdirectories recursively
#   :reverse         - Reverse sort order
#   :sort_by_size    - Sort by file size
#   :sort_by_time    - Sort by modification time
#   :sort_by_ext     - Sort by file extension
#   :unsorted        - Don't sort at all
#   :human_readable  - Human-readable sizes
#   :si              - Use powers of 1000
#   :classify        - Append type indicator
#   :inode           - Show inode numbers
#   :numeric_uid_gid - Numeric user/group IDs
#   :no_group        - Don't show group
#   :directory       - List directories themselves, not contents
#   :dereference     - Follow symlinks

def ls_list(path, opts = {})
  # --- If -d flag is set, list the directory itself, not its contents ----------
  if opts[:directory]
    return [ls_format_entry(path, path, opts)]
  end

  # --- If path is a file, just format that file --------------------------------
  unless File.directory?(path)
    return [ls_format_entry(path, File.basename(path), opts)]
  end

  # --- Read directory entries --------------------------------------------------
  entries = Dir.entries(path)

  # --- Filter hidden entries ---------------------------------------------------
  unless opts[:all] || opts[:almost_all]
    entries = entries.reject { |e| e.start_with?(".") }
  end

  if opts[:almost_all]
    entries = entries.reject { |e| e == "." || e == ".." }
  end

  # --- Sort entries ------------------------------------------------------------
  entries = ls_sort_entries(entries, path, opts)

  # --- Format each entry -------------------------------------------------------
  lines = entries.map do |entry|
    full_path = File.join(path, entry)
    ls_format_entry(full_path, entry, opts)
  end

  # --- Recursive listing -------------------------------------------------------
  if opts[:recursive]
    entries.each do |entry|
      next if entry == "." || entry == ".."
      full_path = File.join(path, entry)
      next unless File.directory?(full_path) && !File.symlink?(full_path)

      lines << ""
      lines << "#{full_path}:"
      lines.concat(ls_list(full_path, opts))
    end
  end

  lines
end

# ---------------------------------------------------------------------------
# Helper: ls_sort_entries
# ---------------------------------------------------------------------------
# Sort directory entries according to the given options.
#
# The sort key depends on the flags:
#   (default) Alphabetical, case-insensitive
#   -S        By file size, largest first
#   -t        By modification time, newest first
#   -X        By file extension
#   -U        Unsorted (directory order)
#
# -r reverses whatever sort order is chosen.

def ls_sort_entries(entries, dir_path, opts = {})
  return entries if opts[:unsorted]

  sorted = if opts[:sort_by_size]
    entries.sort_by { |e| [-File.lstat(File.join(dir_path, e)).size, e.downcase] }
  elsif opts[:sort_by_time]
    entries.sort_by { |e| [-File.lstat(File.join(dir_path, e)).mtime.to_f, e.downcase] }
  elsif opts[:sort_by_ext]
    entries.sort_by { |e| [File.extname(e).downcase, e.downcase] }
  else
    # Default: alphabetical, case-insensitive.
    entries.sort_by { |e| e.downcase }
  end

  opts[:reverse] ? sorted.reverse : sorted
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def ls_main
  begin
    result = CodingAdventures::CliBuilder::Parser.new(LS_SPEC_FILE, ["ls"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "ls: #{err.message}" }
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
    opts = {
      all: result.flags["all"] || false,
      almost_all: result.flags["almost_all"] || false,
      long: result.flags["long"] || result.flags["numeric_uid_gid"] || false,
      human_readable: result.flags["human_readable"] || false,
      si: result.flags["si"] || false,
      reverse: result.flags["reverse"] || false,
      recursive: result.flags["recursive"] || false,
      sort_by_size: result.flags["sort_by_size"] || false,
      sort_by_time: result.flags["sort_by_time"] || false,
      sort_by_ext: result.flags["sort_by_extension"] || false,
      unsorted: result.flags["unsorted"] || false,
      classify: result.flags["classify"] || false,
      inode: result.flags["inode"] || false,
      numeric_uid_gid: result.flags["numeric_uid_gid"] || false,
      no_group: result.flags["no_group"] || false,
      directory: result.flags["directory"] || false,
      dereference: result.flags["dereference"] || false,
    }

    files = result.arguments.fetch("files", ["."])
    files = [files] if files.is_a?(String)
    files = ["."] if files.empty?

    # When listing multiple paths, show headers for each directory.
    show_header = files.length > 1

    files.each_with_index do |path, idx|
      unless File.exist?(path) || File.symlink?(path)
        warn "ls: cannot access '#{path}': No such file or directory"
        next
      end

      if show_header && File.directory?(path)
        puts "" if idx > 0
        puts "#{path}:"
      end

      lines = ls_list(path, opts)
      lines.each { |line| puts line }
    end
  end
end

ls_main if __FILE__ == $PROGRAM_NAME
