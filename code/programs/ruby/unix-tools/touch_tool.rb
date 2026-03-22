#!/usr/bin/env ruby
# frozen_string_literal: true

# touch_tool.rb -- Change file timestamps or create empty files
# ==============================================================
#
# === What This Program Does ===
#
# This is a reimplementation of the GNU `touch` utility. It creates
# files if they don't exist, and updates timestamps on files that do.
#
# === File Timestamps ===
#
# Every file has access time (atime) and modification time (mtime).
# By default, touch sets both to the current time.
#
#   -a   Change only the access time.
#   -m   Change only the modification time.
#   -c   Don't create files that don't exist.
#   -t   Use a specific timestamp in [[CC]YY]MMDDhhmm[.ss] format.
#   -d   Use a date string (ISO 8601-ish).
#   -r   Copy timestamps from a reference file.

require "time"
require "fileutils"
require "coding_adventures_cli_builder"

TOUCH_SPEC_FILE = File.join(File.dirname(__FILE__), "touch.json")

# ---------------------------------------------------------------------------
# Business Logic: parse_timestamp
# ---------------------------------------------------------------------------
# Parse a touch-style timestamp: [[CC]YY]MMDDhhmm[.ss]

def touch_parse_timestamp(stamp)
  # Split off optional seconds.
  main_part, sec_part = stamp.split(".", 2)
  seconds = sec_part ? sec_part.to_i : 0

  now = Time.now

  case main_part.length
  when 8  # MMDDhhmm
    Time.new(now.year, main_part[0, 2].to_i, main_part[2, 2].to_i,
             main_part[4, 2].to_i, main_part[6, 2].to_i, seconds)
  when 10 # YYMMDDhhmm
    year = main_part[0, 2].to_i
    year = year >= 69 ? year + 1900 : year + 2000
    Time.new(year, main_part[2, 2].to_i, main_part[4, 2].to_i,
             main_part[6, 2].to_i, main_part[8, 2].to_i, seconds)
  when 12 # CCYYMMDDhhmm
    Time.new(main_part[0, 4].to_i, main_part[4, 2].to_i, main_part[6, 2].to_i,
             main_part[8, 2].to_i, main_part[10, 2].to_i, seconds)
  end
rescue ArgumentError
  nil
end

# ---------------------------------------------------------------------------
# Business Logic: parse_date_string
# ---------------------------------------------------------------------------
# Parse an ISO 8601-ish date string.

def touch_parse_date(date_str)
  Time.parse(date_str)
rescue ArgumentError
  nil
end

# ---------------------------------------------------------------------------
# Business Logic: touch_file
# ---------------------------------------------------------------------------
# Touch a file: create it or update timestamps.

def touch_touch_file(filepath, no_create:, access_only:, modify_only:, timestamp:)
  unless File.exist?(filepath)
    return true if no_create
    begin
      FileUtils.touch(filepath)
    rescue Errno::ENOENT
      warn "touch: cannot touch '#{filepath}': No such file or directory"
      return false
    rescue Errno::EACCES
      warn "touch: cannot touch '#{filepath}': Permission denied"
      return false
    end
  end

  now = timestamp || Time.now
  stat = File.stat(filepath)

  if access_only
    new_atime = now
    new_mtime = stat.mtime
  elsif modify_only
    new_atime = stat.atime
    new_mtime = now
  else
    new_atime = now
    new_mtime = now
  end

  begin
    File.utime(new_atime, new_mtime, filepath)
  rescue Errno::EACCES
    warn "touch: cannot touch '#{filepath}': Permission denied"
    return false
  end

  true
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def touch_main
  begin
    result = CodingAdventures::CliBuilder::Parser.new(TOUCH_SPEC_FILE, ["touch"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "touch: #{err.message}" }
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
    access_only = result.flags["access_only"] || false
    modify_only = result.flags["modify_only"] || false
    no_create = result.flags["no_create"] || false
    timestamp_str = result.flags["timestamp"]
    date_str = result.flags["date"]
    reference = result.flags["reference"]

    timestamp = nil

    if reference
      unless File.exist?(reference)
        warn "touch: failed to get attributes of '#{reference}': No such file or directory"
        exit 1
      end
      timestamp = File.stat(reference).mtime
    end

    if timestamp_str
      timestamp = touch_parse_timestamp(timestamp_str)
      if timestamp.nil?
        warn "touch: invalid date format '#{timestamp_str}'"
        exit 1
      end
    end

    if date_str
      timestamp = touch_parse_date(date_str)
      if timestamp.nil?
        warn "touch: invalid date format '#{date_str}'"
        exit 1
      end
    end

    files = result.arguments.fetch("files", [])
    files = [files] if files.is_a?(String)

    exit_code = 0
    files.each do |filepath|
      unless touch_touch_file(filepath, no_create: no_create, access_only: access_only,
                                        modify_only: modify_only, timestamp: timestamp)
        exit_code = 1
      end
    end

    exit exit_code
  end
end

touch_main if __FILE__ == $PROGRAM_NAME
