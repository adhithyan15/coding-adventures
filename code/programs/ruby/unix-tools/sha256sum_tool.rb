#!/usr/bin/env ruby
# frozen_string_literal: true

# sha256sum_tool.rb -- Compute and check SHA256 message digest
# ==============================================================
#
# === What This Program Does ===
#
# This is a reimplementation of the GNU `sha256sum` utility. It computes
# SHA-256 message digests (256-bit cryptographic hashes) for files, or
# verifies previously computed digests.
#
# === Computing Digests ===
#
#     $ sha256sum file1.txt
#     e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855  file1.txt
#
# Each line shows the 64-character hex-encoded SHA-256 hash followed
# by two spaces and the filename.
#
# === SHA-256 vs MD5 ===
#
# SHA-256 is part of the SHA-2 family designed by the NSA. Compared
# to MD5:
#   - 256-bit output (64 hex chars) vs 128-bit (32 hex chars)
#   - No known practical collisions (MD5 has known collisions)
#   - Slower to compute (more rounds, more complex mixing)
#   - Suitable for security applications
#
# === Checking Digests (-c) ===
#
# In check mode, sha256sum reads a file containing previously computed
# checksums and verifies them, same as md5sum.
#
# === Implementation ===
#
# We use Ruby's `Digest::SHA256` from the standard library. The API
# is identical to Digest::MD5 -- only the algorithm and output length
# differ.

require "digest"
require "coding_adventures_cli_builder"

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

SHA256SUM_SPEC_FILE = File.join(File.dirname(__FILE__), "sha256sum.json")

# ---------------------------------------------------------------------------
# Business Logic: compute_sha256
# ---------------------------------------------------------------------------
# Compute the SHA-256 hash of a file or IO stream.
#
# We read in 8KB chunks for memory efficiency, using incremental
# digest updates.
#
# Parameters:
#   io - An IO object to read from
#
# Returns: The hex-encoded SHA-256 digest string (64 characters).

def compute_sha256(io)
  digest = Digest::SHA256.new
  while (chunk = io.read(8192))
    digest.update(chunk)
  end
  digest.hexdigest
end

# ---------------------------------------------------------------------------
# Business Logic: format_sha256_line
# ---------------------------------------------------------------------------
# Format a checksum line for output.
#
# Parameters:
#   hash     - The hex digest string
#   filename - The file path
#   binary   - Whether binary mode indicator should be used
#
# Returns: Formatted string.

def format_sha256_line(hash, filename, binary)
  separator = binary ? " *" : "  "
  "#{hash}#{separator}#{filename}"
end

# ---------------------------------------------------------------------------
# Business Logic: check_sha256_checksums
# ---------------------------------------------------------------------------
# Read a checksum file and verify each SHA-256 entry.
#
# The format is the same as md5sum check files, but with 64-character
# hex hashes instead of 32.
#
# Parameters:
#   io    - IO to read checksum entries from
#   flags - CLI flags hash
#
# Returns: true if all checks passed, false otherwise.

def check_sha256_checksums(io, flags)
  quiet = flags["quiet"]
  status_only = flags["status"]
  strict = flags["strict"]
  show_warn = flags["warn"]

  total = 0
  failed = 0
  malformed = 0

  io.each_line do |line|
    line = line.chomp

    # Parse: 64 hex chars + separator + filename
    match = line.match(/\A([0-9a-fA-F]{64})(  | \*)(.+)\z/)

    unless match
      malformed += 1
      if show_warn
        warn "sha256sum: WARNING: #{malformed} line is improperly formatted"
      end
      next
    end

    expected_hash = match[1].downcase
    filename = match[3]
    total += 1

    begin
      actual_hash = File.open(filename, "rb") { |f| compute_sha256(f) }

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

  if failed > 0 && !status_only
    warn "sha256sum: WARNING: #{failed} computed checksum did NOT match"
  end

  return false if strict && malformed > 0

  failed == 0
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def sha256sum_main
  # --- Step 1: Parse arguments ---------------------------------------------
  begin
    result = CodingAdventures::CliBuilder::Parser.new(SHA256SUM_SPEC_FILE, ["sha256sum"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "sha256sum: #{err.message}" }
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
                 warn "sha256sum: #{filename}: No such file or directory"
                 all_ok = false
                 next
               end
             end

        all_ok = false unless check_sha256_checksums(io, result.flags)
        io.close if io != $stdin
      end
      exit(all_ok ? 0 : 1)
    else
      # --- Compute mode ----------------------------------------------------
      files.each do |filename|
        if filename == "-"
          hash = compute_sha256($stdin)
          print format_sha256_line(hash, "-", binary) + line_end
        else
          begin
            hash = File.open(filename, "rb") { |f| compute_sha256(f) }
            print format_sha256_line(hash, filename, binary) + line_end
          rescue Errno::ENOENT
            warn "sha256sum: #{filename}: No such file or directory"
          rescue Errno::EISDIR
            warn "sha256sum: #{filename}: Is a directory"
          end
        end
      end
    end
  end
end

# Only run main when this file is executed directly.
sha256sum_main if __FILE__ == $PROGRAM_NAME
