#!/usr/bin/env ruby
# frozen_string_literal: true

# md5sum_tool.rb -- Compute and check MD5 message digest
# ========================================================
#
# === What This Program Does ===
#
# This is a reimplementation of the GNU `md5sum` utility. It computes
# MD5 message digests (128-bit cryptographic hashes) for files, or
# verifies previously computed digests.
#
# === Computing Digests ===
#
#     $ md5sum file1.txt file2.txt
#     d41d8cd98f00b204e9800998ecf8427e  file1.txt
#     e99a18c428cb38d5f260853678922e03  file2.txt
#
# Each line shows the hex-encoded MD5 hash followed by two spaces and
# the filename. In binary mode (-b), the separator is " *" instead of
# "  " (space-asterisk vs two spaces).
#
# === Checking Digests (-c) ===
#
# In check mode, md5sum reads a file containing previously computed
# checksums and verifies them:
#
#     $ md5sum -c checksums.txt
#     file1.txt: OK
#     file2.txt: FAILED
#
# The input format must match the output format: hash followed by
# two spaces (or " *") and the filename.
#
# === MD5 Algorithm Overview ===
#
# MD5 (Message Digest 5) was designed by Ronald Rivest in 1991. It
# produces a 128-bit (16-byte) hash value, typically displayed as a
# 32-character hexadecimal string.
#
# IMPORTANT: MD5 is cryptographically broken and should NOT be used
# for security purposes. It's still useful for:
# - File integrity checking (detecting corruption)
# - Non-security checksums (build systems, caches)
# - Legacy compatibility
#
# We use Ruby's `Digest::MD5` from the standard library, which wraps
# the OpenSSL implementation.

require "digest"
require "coding_adventures_cli_builder"

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

MD5SUM_SPEC_FILE = File.join(File.dirname(__FILE__), "md5sum.json")

# ---------------------------------------------------------------------------
# Business Logic: compute_md5
# ---------------------------------------------------------------------------
# Compute the MD5 hash of a file or IO stream.
#
# We read in 8KB chunks to handle large files without loading them
# entirely into memory. The Digest API supports incremental updates
# via the `update` method.
#
# Parameters:
#   io - An IO object to read from
#
# Returns: The hex-encoded MD5 digest string (32 characters).

def compute_md5(io)
  digest = Digest::MD5.new
  while (chunk = io.read(8192))
    digest.update(chunk)
  end
  digest.hexdigest
end

# ---------------------------------------------------------------------------
# Business Logic: format_checksum_line
# ---------------------------------------------------------------------------
# Format a checksum line for output.
#
# The format is:
#   HASH  FILENAME     (text mode: two spaces)
#   HASH *FILENAME     (binary mode: space + asterisk)
#
# Parameters:
#   hash     - The hex digest string
#   filename - The file path
#   binary   - Whether binary mode indicator should be used
#
# Returns: Formatted string.

def format_checksum_line(hash, filename, binary)
  separator = binary ? " *" : "  "
  "#{hash}#{separator}#{filename}"
end

# ---------------------------------------------------------------------------
# Business Logic: check_checksums
# ---------------------------------------------------------------------------
# Read a checksum file and verify each entry.
#
# The checksum file format is:
#   HASH  FILENAME
#   HASH *FILENAME
#
# For each entry, we compute the file's hash and compare it to the
# stored hash. We track success/failure counts and report them.
#
# Parameters:
#   io     - IO to read checksum entries from
#   flags  - CLI flags hash (quiet, status, strict, warn)
#
# Returns: true if all checks passed, false otherwise.

def check_checksums(io, flags)
  quiet = flags["quiet"]
  status_only = flags["status"]
  strict = flags["strict"]
  show_warn = flags["warn"]

  total = 0
  failed = 0
  malformed = 0

  io.each_line do |line|
    line = line.chomp

    # Parse the checksum line
    # Format: HASH  FILENAME or HASH *FILENAME
    match = line.match(/\A([0-9a-fA-F]{32})(  | \*)(.+)\z/)

    unless match
      malformed += 1
      if show_warn
        warn "md5sum: WARNING: #{malformed} line is improperly formatted"
      end
      next
    end

    expected_hash = match[1].downcase
    filename = match[3]
    total += 1

    # Compute the actual hash
    begin
      actual_hash = File.open(filename, "rb") { |f| compute_md5(f) }

      if actual_hash == expected_hash
        puts "#{filename}: OK" unless quiet || status_only
      else
        puts "#{filename}: FAILED" unless status_only
        failed += 1
      end
    rescue Errno::ENOENT
      puts "#{filename}: FAILED open or read" unless status_only
      failed += 1
    end
  end

  # Summary
  if failed > 0 && !status_only
    warn "md5sum: WARNING: #{failed} computed checksum did NOT match"
  end

  if strict && malformed > 0
    return false
  end

  failed == 0
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def md5sum_main
  # --- Step 1: Parse arguments ---------------------------------------------
  begin
    result = CodingAdventures::CliBuilder::Parser.new(MD5SUM_SPEC_FILE, ["md5sum"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "md5sum: #{err.message}" }
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
    files = result.arguments.fetch("files", ["-"])
    binary = result.flags["binary"]
    line_end = result.flags["zero"] ? "\0" : "\n"

    if result.flags["check"]
      # --- Check mode ------------------------------------------------------
      all_ok = true
      files.each do |filename|
        io = if filename == "-"
               $stdin
             else
               begin
                 File.open(filename, "r")
               rescue Errno::ENOENT
                 warn "md5sum: #{filename}: No such file or directory"
                 all_ok = false
                 next
               end
             end

        all_ok = false unless check_checksums(io, result.flags)
        io.close if io != $stdin
      end
      exit(all_ok ? 0 : 1)
    else
      # --- Compute mode ----------------------------------------------------
      files.each do |filename|
        if filename == "-"
          hash = compute_md5($stdin)
          print format_checksum_line(hash, "-", binary) + line_end
        else
          begin
            hash = File.open(filename, "rb") { |f| compute_md5(f) }
            print format_checksum_line(hash, filename, binary) + line_end
          rescue Errno::ENOENT
            warn "md5sum: #{filename}: No such file or directory"
          rescue Errno::EISDIR
            warn "md5sum: #{filename}: Is a directory"
          end
        end
      end
    end
  end
end

# Only run main when this file is executed directly.
md5sum_main if __FILE__ == $PROGRAM_NAME
