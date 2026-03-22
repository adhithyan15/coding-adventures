#!/usr/bin/env ruby
# frozen_string_literal: true

# ln_tool.rb -- Make links between files
# ========================================
#
# === What This Program Does ===
#
# This is a reimplementation of the GNU `ln` utility. It creates links
# between files -- either hard links (default) or symbolic links (-s).
#
# === Hard vs Symbolic Links ===
#
# Hard links: Two directory entries pointing to the same inode (data).
# Both names are equally "real". Cannot cross filesystems.
#
# Symbolic links (-s): A small file containing the path to another file.
# Can cross filesystems, can link to directories, but break if target moves.
#
# === Flags ===
#
#   -s   Create symbolic links instead of hard links
#   -f   Remove existing destination files
#   -v   Print the name of each linked file
#   -r   Create relative symbolic links
#   -n   Treat link name as normal file even if it's a symlink to a directory
#   -T   Treat link name as a normal file always

require "pathname"
require "coding_adventures_cli_builder"

LN_SPEC_FILE = File.join(File.dirname(__FILE__), "ln.json")

# ---------------------------------------------------------------------------
# Business Logic: make_link
# ---------------------------------------------------------------------------

def ln_make_link(target, link_name, symbolic:, force:, verbose:, relative:, no_dereference:)
  # If the link_name is a directory (and we're not using -n/-T),
  # create the link inside that directory.
  if !no_dereference && File.directory?(link_name)
    link_name = File.join(link_name, File.basename(target))
  end

  # Remove existing file if force is enabled.
  if force && (File.exist?(link_name) || File.symlink?(link_name))
    begin
      File.unlink(link_name)
    rescue SystemCallError => e
      warn "ln: cannot remove '#{link_name}': #{e.message}"
      return false
    end
  end

  # Compute relative path for symlinks if requested.
  actual_target = target
  if symbolic && relative
    link_dir = File.dirname(File.expand_path(link_name))
    actual_target = Pathname.new(File.expand_path(target)).relative_path_from(Pathname.new(link_dir)).to_s
  end

  begin
    if symbolic
      File.symlink(actual_target, link_name)
    else
      File.link(target, link_name)
    end
  rescue Errno::EEXIST
    link_type = symbolic ? "symbolic link" : "hard link"
    warn "ln: failed to create #{link_type} '#{link_name}': File exists"
    return false
  rescue Errno::ENOENT
    link_type = symbolic ? "symbolic link" : "hard link"
    warn "ln: failed to create #{link_type} '#{link_name}': No such file or directory"
    return false
  rescue Errno::EACCES
    link_type = symbolic ? "symbolic link" : "hard link"
    warn "ln: failed to create #{link_type} '#{link_name}': Permission denied"
    return false
  end

  if verbose
    arrow = symbolic ? " -> " : " => "
    puts "'#{link_name}'#{arrow}'#{actual_target}'"
  end

  true
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def ln_main
  begin
    result = CodingAdventures::CliBuilder::Parser.new(LN_SPEC_FILE, ["ln"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "ln: #{err.message}" }
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
    symbolic = result.flags["symbolic"] || false
    force = result.flags["force"] || false
    verbose = result.flags["verbose"] || false
    relative = result.flags["relative"] || false
    no_dereference = result.flags["no_dereference"] || false
    no_target_dir = result.flags["no_target_directory"] || false

    targets = result.arguments.fetch("targets", [])
    targets = [targets] if targets.is_a?(String)

    if targets.empty?
      warn "ln: missing file operand"
      exit 1
    end

    if targets.length == 1
      target = targets[0]
      link_name = File.basename(target)
      success = ln_make_link(target, link_name, symbolic: symbolic, force: force,
                             verbose: verbose, relative: relative,
                             no_dereference: no_target_dir || no_dereference)
      exit(success ? 0 : 1)
    end

    if targets.length == 2 && (no_target_dir || !File.directory?(targets[-1]))
      success = ln_make_link(targets[0], targets[1], symbolic: symbolic, force: force,
                             verbose: verbose, relative: relative,
                             no_dereference: no_target_dir || no_dereference)
      exit(success ? 0 : 1)
    end

    destination = targets[-1]
    unless File.directory?(destination)
      warn "ln: target '#{destination}' is not a directory"
      exit 1
    end

    exit_code = 0
    targets[0...-1].each do |target|
      link_name = File.join(destination, File.basename(target))
      unless ln_make_link(target, link_name, symbolic: symbolic, force: force,
                          verbose: verbose, relative: relative, no_dereference: true)
        exit_code = 1
      end
    end

    exit exit_code
  end
end

ln_main if __FILE__ == $PROGRAM_NAME
