#!/usr/bin/env ruby
# frozen_string_literal: true

# realpath_tool.rb -- Print the resolved absolute path
# =====================================================
#
# === What This Program Does ===
#
# This is a reimplementation of the GNU `realpath` utility. It resolves
# each given path to an absolute path by following symlinks and removing
# `.` and `..` components.
#
# === Modes ===
#
#   Default: Resolve symlinks; final component need not exist.
#   -e:      All components must exist (canonicalize-existing).
#   -m:      No component need exist (canonicalize-missing).
#   -s:      Don't follow symlinks (just make path absolute).
#
# === Relative Output ===
#
#   --relative-to=DIR:  Print path relative to DIR.
#   --relative-base=DIR: Print relative only if under DIR.

require "pathname"
require "coding_adventures_cli_builder"

REALPATH_SPEC_FILE = File.join(File.dirname(__FILE__), "realpath.json")

# ---------------------------------------------------------------------------
# Business Logic: resolve_path
# ---------------------------------------------------------------------------

def realpath_resolve(filepath, canonicalize_existing:, canonicalize_missing:, no_symlinks:)
  if no_symlinks
    return File.expand_path(filepath)
  end

  if canonicalize_existing
    begin
      File.realpath(filepath)
    rescue Errno::ENOENT
      nil
    end
  elsif canonicalize_missing
    # Resolve what we can. Ruby's File.expand_path handles .. and .
    # but doesn't resolve symlinks for nonexistent paths.
    begin
      File.realpath(filepath)
    rescue Errno::ENOENT
      # For missing paths, resolve the existing prefix and append the rest.
      parts = filepath.split("/")
      resolved = ""
      parts.each_with_index do |part, i|
        candidate = resolved.empty? ? part : "#{resolved}/#{part}"
        candidate = "/" if candidate.empty?
        begin
          resolved = File.realpath(candidate)
        rescue Errno::ENOENT
          remaining = parts[i..].join("/")
          return resolved.empty? ? File.expand_path(filepath) : "#{resolved}/#{remaining}"
        end
      end
      resolved
    end
  else
    # Default mode: resolve symlinks, final component need not exist.
    begin
      File.realpath(filepath)
    rescue Errno::ENOENT
      # Try resolving the parent, then append the basename.
      parent = File.dirname(filepath)
      base = File.basename(filepath)
      begin
        "#{File.realpath(parent)}/#{base}"
      rescue Errno::ENOENT
        File.expand_path(filepath)
      end
    end
  end
end

# ---------------------------------------------------------------------------
# Business Logic: make_relative
# ---------------------------------------------------------------------------

def realpath_make_relative(resolved, relative_to:, relative_base:)
  if relative_to
    base = File.realpath(relative_to) rescue File.expand_path(relative_to)
    return Pathname.new(resolved).relative_path_from(Pathname.new(base)).to_s
  end

  if relative_base
    base = File.realpath(relative_base) rescue File.expand_path(relative_base)
    if resolved.start_with?("#{base}/") || resolved == base
      return Pathname.new(resolved).relative_path_from(Pathname.new(base)).to_s
    end
    return resolved
  end

  resolved
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def realpath_main
  begin
    result = CodingAdventures::CliBuilder::Parser.new(REALPATH_SPEC_FILE, ["realpath"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "realpath: #{err.message}" }
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
    canonicalize_existing = result.flags["canonicalize_existing"] || false
    canonicalize_missing = result.flags["canonicalize_missing"] || false
    no_symlinks = result.flags["no_symlinks"] || false
    quiet = result.flags["quiet"] || false
    relative_to = result.flags["relative_to"]
    relative_base = result.flags["relative_base"]
    zero = result.flags["zero"] || false

    terminator = zero ? "\0" : "\n"

    files = result.arguments.fetch("files", [])
    files = [files] if files.is_a?(String)

    exit_code = 0
    files.each do |filepath|
      resolved = realpath_resolve(filepath,
                                  canonicalize_existing: canonicalize_existing,
                                  canonicalize_missing: canonicalize_missing,
                                  no_symlinks: no_symlinks)

      if resolved.nil?
        warn "realpath: #{filepath}: No such file or directory" unless quiet
        exit_code = 1
        next
      end

      output = realpath_make_relative(resolved, relative_to: relative_to, relative_base: relative_base)
      $stdout.write("#{output}#{terminator}")
    end

    exit exit_code
  end
end

realpath_main if __FILE__ == $PROGRAM_NAME
