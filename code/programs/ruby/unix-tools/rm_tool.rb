#!/usr/bin/env ruby
# frozen_string_literal: true

# rm_tool.rb -- Remove files or directories
# ===========================================
#
# === What This Program Does ===
#
# This is a reimplementation of the GNU `rm` utility. It removes files
# and directories. Unlike `rmdir`, `rm -r` can remove entire trees.
#
# === Safety Features ===
#
#   --preserve-root  Refuses to operate on '/' (default)
#   -i               Prompts before each removal
#   -I               Prompts once for bulk operations
#
# === Flags ===
#
#   -f   Force: ignore nonexistent files, never prompt
#   -r   Recursive: remove directories and their contents
#   -d   Remove empty directories
#   -v   Verbose: explain what is being done

require "fileutils"
require "coding_adventures_cli_builder"

RM_SPEC_FILE = File.join(File.dirname(__FILE__), "rm.json")

# ---------------------------------------------------------------------------
# Business Logic: confirm_removal
# ---------------------------------------------------------------------------

def rm_confirm(prompt)
  $stderr.print prompt
  response = $stdin.gets&.strip&.downcase
  %w[y yes].include?(response)
end

# ---------------------------------------------------------------------------
# Business Logic: remove_file
# ---------------------------------------------------------------------------

def rm_remove_file(filepath, force:, interactive:, recursive:, verbose:, dir_flag:, preserve_root:)
  # Safety check.
  if preserve_root && File.expand_path(filepath) == "/"
    warn "rm: it is dangerous to operate recursively on '/'"
    warn "rm: use --no-preserve-root to override this failsafe"
    return false
  end

  # Check existence.
  unless File.exist?(filepath) || File.symlink?(filepath)
    warn "rm: cannot remove '#{filepath}': No such file or directory" unless force
    return force
  end

  # Handle directories.
  if File.directory?(filepath) && !File.symlink?(filepath)
    if recursive
      if interactive && !force
        return true unless rm_confirm("rm: descend into directory '#{filepath}'? ")
      end
      begin
        FileUtils.rm_rf(filepath)
      rescue Errno::EACCES
        warn "rm: cannot remove '#{filepath}': Permission denied"
        return false
      end
      puts "removed directory '#{filepath}'" if verbose
      return true
    end

    if dir_flag
      begin
        Dir.rmdir(filepath)
      rescue Errno::ENOTEMPTY
        warn "rm: cannot remove '#{filepath}': Directory not empty"
        return false
      end
      puts "removed directory '#{filepath}'" if verbose
      return true
    end

    warn "rm: cannot remove '#{filepath}': Is a directory"
    return false
  end

  # Handle regular files and symlinks.
  if interactive && !force
    return true unless rm_confirm("rm: remove file '#{filepath}'? ")
  end

  begin
    File.unlink(filepath)
  rescue Errno::EACCES
    warn "rm: cannot remove '#{filepath}': Permission denied" unless force
    return false
  rescue SystemCallError => e
    warn "rm: cannot remove '#{filepath}': #{e.message}" unless force
    return false
  end

  puts "removed '#{filepath}'" if verbose
  true
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def rm_main
  begin
    result = CodingAdventures::CliBuilder::Parser.new(RM_SPEC_FILE, ["rm"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "rm: #{err.message}" }
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
    force = result.flags["force"] || false
    interactive = result.flags["interactive"] || false
    interactive_once = result.flags["interactive_once"] || false
    recursive = result.flags["recursive"] || false
    verbose = result.flags["verbose"] || false
    dir_flag = result.flags["dir"] || false
    preserve_root = result.flags.fetch("preserve_root", true)

    files = result.arguments.fetch("files", [])
    files = [files] if files.is_a?(String)

    if interactive_once && !force
      if files.length > 3 || recursive
        unless rm_confirm("rm: remove #{files.length} argument#{files.length != 1 ? "s" : ""}? ")
          exit 0
        end
      end
    end

    exit_code = 0
    files.each do |filepath|
      unless rm_remove_file(filepath, force: force, interactive: interactive,
                            recursive: recursive, verbose: verbose,
                            dir_flag: dir_flag, preserve_root: preserve_root)
        exit_code = 1
      end
    end

    exit exit_code
  end
end

rm_main if __FILE__ == $PROGRAM_NAME
