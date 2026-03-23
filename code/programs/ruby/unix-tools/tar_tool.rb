#!/usr/bin/env ruby
# frozen_string_literal: true

# tar_tool.rb -- An archiving utility
# ======================================
#
# === What This Program Does ===
#
# This is a reimplementation of the POSIX `tar` utility. It creates,
# extracts, and lists tape archives (tar files). The tar format bundles
# multiple files into a single archive file, preserving directory
# structure, permissions, and timestamps.
#
# === How tar Works ===
#
#     $ tar cf archive.tar file1 file2 dir/   # create an archive
#     $ tar xf archive.tar                     # extract all files
#     $ tar tf archive.tar                     # list archive contents
#     $ tar xf archive.tar -C /tmp/           # extract to /tmp/
#     $ tar cvf archive.tar dir/              # create with verbose output
#
# === The tar Format ===
#
# A tar file consists of a series of 512-byte blocks:
#
#   [header block][data blocks...][header block][data blocks...]...[end-of-archive]
#
# Each file entry starts with a 512-byte header containing:
#   - Filename (100 bytes)
#   - File mode (8 bytes, octal ASCII)
#   - Owner UID (8 bytes, octal)
#   - Group GID (8 bytes, octal)
#   - File size (12 bytes, octal)
#   - Modification time (12 bytes, octal, Unix epoch)
#   - Checksum (8 bytes)
#   - Type flag (1 byte: '0'=file, '5'=directory, '2'=symlink)
#   - Link name (100 bytes)
#   - Magic ("ustar\0" for POSIX)
#   - ... (additional fields)
#
# The data follows immediately after the header, padded to 512-byte
# boundary with null bytes.
#
# The archive ends with two consecutive 512-byte blocks of zeros.
#
# === Our Implementation ===
#
# We use Ruby's built-in Gem::Package::TarWriter and TarReader from
# RubyGems, which implement the POSIX tar format. This gives us a
# correct, well-tested implementation without external dependencies.

require "rubygems/package"
require "fileutils"
require "coding_adventures_cli_builder"

TAR_SPEC_FILE = File.join(File.dirname(__FILE__), "tar.json")

# ---------------------------------------------------------------------------
# Business Logic: tar_create
# ---------------------------------------------------------------------------
# Create a tar archive from a list of files/directories.
#
# Parameters:
#   archive_path - Path to the output archive file (or nil for stdout)
#   files        - Array of file/directory paths to include
#   opts         - Hash with :verbose, :directory (base dir)
#
# Returns: [messages, success?]
#   messages is an array of verbose output lines

def tar_create(archive_path, files, opts = {})
  messages = []
  base_dir = opts[:directory]

  io = if archive_path
         File.open(archive_path, "wb")
       else
         $stdout
       end

  begin
    Gem::Package::TarWriter.new(io) do |tar|
      files.each do |file_path|
        full_path = base_dir ? File.join(base_dir, file_path) : file_path

        if File.directory?(full_path)
          tar_add_directory(tar, full_path, file_path, opts, messages)
        elsif File.file?(full_path)
          tar_add_file(tar, full_path, file_path, opts, messages)
        else
          messages << "tar: #{file_path}: Cannot stat: No such file or directory"
        end
      end
    end
  ensure
    io.close if archive_path && io != $stdout
  end

  [messages, true]
end

# ---------------------------------------------------------------------------
# Helper: tar_add_file
# ---------------------------------------------------------------------------
# Add a single file to the tar archive.
#
# The TarWriter#add_file method takes:
#   - name: the path to store in the archive
#   - mode: the file permissions
#   - block: yields an IO to write the file contents into

def tar_add_file(tar, full_path, archive_path, opts, messages)
  stat = File.stat(full_path)
  mode = stat.mode & 0o7777
  content = File.binread(full_path)

  tar.add_file_simple(archive_path, mode, content.bytesize) do |io|
    io.write(content)
  end

  messages << archive_path if opts[:verbose]
end

# ---------------------------------------------------------------------------
# Helper: tar_add_directory
# ---------------------------------------------------------------------------
# Recursively add a directory and its contents to the tar archive.
#
# Directories are stored as entries with type '5' (directory) and
# no data blocks. The trailing slash in the name is the convention
# that marks an entry as a directory.

def tar_add_directory(tar, full_path, archive_path, opts, messages)
  # Add the directory entry itself
  dir_name = archive_path.end_with?("/") ? archive_path : "#{archive_path}/"
  tar.mkdir(dir_name, File.stat(full_path).mode & 0o7777)
  messages << archive_path if opts[:verbose]

  # Recursively add contents
  Dir.entries(full_path).sort.each do |entry|
    next if entry == "." || entry == ".."
    child_full = File.join(full_path, entry)
    child_archive = File.join(archive_path, entry)

    # Check exclusion patterns
    if opts[:exclude]
      excludes = opts[:exclude].is_a?(Array) ? opts[:exclude] : [opts[:exclude]]
      next if excludes.any? { |pat| File.fnmatch?(pat, entry) }
    end

    if File.directory?(child_full)
      tar_add_directory(tar, child_full, child_archive, opts, messages)
    elsif File.file?(child_full)
      tar_add_file(tar, child_full, child_archive, opts, messages)
    end
  end
end

# ---------------------------------------------------------------------------
# Business Logic: tar_list
# ---------------------------------------------------------------------------
# List the contents of a tar archive.
#
# Parameters:
#   archive_path - Path to the archive file
#   filter_files - Optional array of specific files to list (nil = all)
#   opts         - Hash with :verbose
#
# Returns: [lines, success?]
#   lines is an array of output strings (one per entry)

def tar_list(archive_path, filter_files = nil, opts = {})
  lines = []

  File.open(archive_path, "rb") do |io|
    Gem::Package::TarReader.new(io) do |tar|
      tar.each do |entry|
        name = entry.full_name
        # Strip leading "./" if present
        name = name.sub(%r{\A\./}, "")
        next if name.empty?

        # Apply filter if specified
        if filter_files && !filter_files.empty?
          next unless filter_files.any? { |f| name.start_with?(f) }
        end

        if opts[:verbose]
          # Verbose listing: permissions, size, date, name
          mode = entry.header.mode
          size = entry.header.size
          mtime_val = entry.header.mtime
          perm_str = tar_format_permissions(mode, entry.directory?)
          # mtime may be a Time or an Integer (Unix epoch seconds)
          mtime_time = mtime_val.is_a?(Time) ? mtime_val : Time.at(mtime_val)
          time_str = mtime_time.strftime("%Y-%m-%d %H:%M")
          lines << "#{perm_str} #{size.to_s.rjust(8)} #{time_str} #{name}"
        else
          lines << name
        end
      end
    end
  end

  [lines, true]
rescue Errno::ENOENT
  [["tar: #{archive_path}: Cannot open: No such file or directory"], false]
end

# ---------------------------------------------------------------------------
# Helper: tar_format_permissions
# ---------------------------------------------------------------------------
# Format a numeric mode as a permission string like "rwxr-xr-x".

def tar_format_permissions(mode, is_dir)
  result = is_dir ? "d" : "-"

  # User permissions
  result += (mode & 0o400) != 0 ? "r" : "-"
  result += (mode & 0o200) != 0 ? "w" : "-"
  result += (mode & 0o100) != 0 ? "x" : "-"

  # Group permissions
  result += (mode & 0o040) != 0 ? "r" : "-"
  result += (mode & 0o020) != 0 ? "w" : "-"
  result += (mode & 0o010) != 0 ? "x" : "-"

  # Other permissions
  result += (mode & 0o004) != 0 ? "r" : "-"
  result += (mode & 0o002) != 0 ? "w" : "-"
  result += (mode & 0o001) != 0 ? "x" : "-"

  result
end

# ---------------------------------------------------------------------------
# Business Logic: tar_extract
# ---------------------------------------------------------------------------
# Extract files from a tar archive.
#
# Parameters:
#   archive_path     - Path to the archive file
#   output_dir       - Directory to extract into (default: current dir)
#   filter_files     - Optional array of specific files to extract
#   opts             - Hash with :verbose, :keep_old_files,
#                      :preserve_permissions, :strip_components
#
# Returns: [messages, success?]

def tar_extract(archive_path, output_dir = ".", filter_files = nil, opts = {})
  messages = []
  strip = opts[:strip_components] || 0

  File.open(archive_path, "rb") do |io|
    Gem::Package::TarReader.new(io) do |tar|
      tar.each do |entry|
        name = entry.full_name
        name = name.sub(%r{\A\./}, "")
        next if name.empty?

        # Apply filter if specified
        if filter_files && !filter_files.empty?
          next unless filter_files.any? { |f| name.start_with?(f) }
        end

        # Strip leading path components
        if strip > 0
          parts = name.split("/")
          parts = parts[strip..] || []
          name = parts.join("/")
          next if name.empty?
        end

        dest = File.join(output_dir, name)

        if entry.directory?
          FileUtils.mkdir_p(dest)
          messages << name if opts[:verbose]
        elsif entry.file?
          # Don't overwrite if -k is set
          if opts[:keep_old_files] && File.exist?(dest)
            messages << "tar: #{name}: Already exists, skipping" if opts[:verbose]
            next
          end

          # Ensure parent directory exists
          FileUtils.mkdir_p(File.dirname(dest))

          File.open(dest, "wb") do |f|
            f.write(entry.read)
          end

          # Restore permissions if requested
          if opts[:preserve_permissions]
            File.chmod(entry.header.mode & 0o7777, dest)
          end

          messages << name if opts[:verbose]
        end
      end
    end
  end

  [messages, true]
rescue Errno::ENOENT
  [["tar: #{archive_path}: Cannot open: No such file or directory"], false]
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def tar_main
  begin
    result = CodingAdventures::CliBuilder::Parser.new(TAR_SPEC_FILE, ["tar"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "tar: #{err.message}" }
    exit 2
  end

  case result
  when CodingAdventures::CliBuilder::HelpResult
    puts result.text
    exit 0
  when CodingAdventures::CliBuilder::VersionResult
    puts result.version
    exit 0
  when CodingAdventures::CliBuilder::ParseResult
    files = result.arguments.fetch("files", [])
    files = [files] if files.is_a?(String)
    archive = result.flags["file"]

    opts = {
      verbose: result.flags["verbose"] || false,
      directory: result.flags["directory"],
      exclude: result.flags["exclude"],
      keep_old_files: result.flags["keep_old_files"] || false,
      preserve_permissions: result.flags["preserve_permissions"] || false,
      strip_components: result.flags["strip_components"],
    }

    if result.flags["create"]
      unless archive
        warn "tar: Refusing to create archive to stdout without -f"
        exit 2
      end
      messages, _success = tar_create(archive, files, opts)
      messages.each { |m| puts m }
    elsif result.flags["list"]
      unless archive
        warn "tar: Refusing to read archive from stdin without -f"
        exit 2
      end
      lines, success = tar_list(archive, files.empty? ? nil : files, opts)
      lines.each { |l| puts l }
      exit(success ? 0 : 2)
    elsif result.flags["extract"]
      unless archive
        warn "tar: Refusing to read archive from stdin without -f"
        exit 2
      end
      output_dir = opts[:directory] || "."
      messages, success = tar_extract(archive, output_dir, files.empty? ? nil : files, opts)
      messages.each { |m| puts m }
      exit(success ? 0 : 2)
    else
      warn "tar: You must specify one of -c, -x, -t"
      exit 2
    end
  end
end

tar_main if __FILE__ == $PROGRAM_NAME
