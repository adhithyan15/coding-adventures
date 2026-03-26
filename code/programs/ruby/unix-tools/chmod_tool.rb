#!/usr/bin/env ruby
# frozen_string_literal: true

# chmod_tool.rb -- Change file mode bits
# ========================================
#
# === What This Program Does ===
#
# This is a reimplementation of the POSIX `chmod` utility. It changes
# the permission bits of files and directories. Permissions can be
# specified in octal notation (e.g., 755) or symbolic notation
# (e.g., u+rwx,go+rx).
#
# === How chmod Works ===
#
#     $ chmod 755 script.sh            # octal: rwxr-xr-x
#     $ chmod u+x script.sh            # add execute for user
#     $ chmod go-w file.txt            # remove write for group+others
#     $ chmod a=r file.txt             # set read-only for all
#     $ chmod -R 755 directory/        # recursive mode change
#
# === Permission Bits ===
#
# Unix permissions are stored as a 12-bit integer:
#
#   Bit layout (octal):  [special][user][group][other]
#
#   Special bits:
#     4000 = setuid (SUID) -- run as file owner
#     2000 = setgid (SGID) -- run as file group
#     1000 = sticky bit    -- restricted deletion
#
#   Permission triplets (repeated for user, group, other):
#     4 = read (r)
#     2 = write (w)
#     1 = execute (x)
#
#   Example: 0755 = 0b_111_101_101
#     user:  rwx (7)
#     group: r-x (5)
#     other: r-x (5)
#
# === Symbolic Mode Syntax ===
#
# A symbolic mode has the form: [who][op][perm]
#
#   who:  u (user), g (group), o (other), a (all = ugo)
#   op:   + (add), - (remove), = (set exactly)
#   perm: r (read), w (write), x (execute), X (execute if directory
#         or already executable), s (setuid/setgid), t (sticky)
#
# Multiple clauses can be comma-separated: u+rwx,go+rx

require "fileutils"
require "coding_adventures_cli_builder"

CHMOD_SPEC_FILE = File.join(File.dirname(__FILE__), "chmod.json")

# ---------------------------------------------------------------------------
# Business Logic: chmod_parse_octal
# ---------------------------------------------------------------------------
# Parse an octal mode string (e.g., "755") into an integer.
#
# Returns: [mode_integer, true] on success, [nil, false] on failure.

def chmod_parse_octal(mode_str)
  return [nil, false] unless mode_str.match?(/\A[0-7]{1,4}\z/)
  [mode_str.to_i(8), true]
end

# ---------------------------------------------------------------------------
# Business Logic: chmod_parse_symbolic
# ---------------------------------------------------------------------------
# Parse a symbolic mode string (e.g., "u+rwx,go-w") and apply it to
# an existing mode.
#
# The symbolic mode grammar:
#   mode    ::= clause (',' clause)*
#   clause  ::= [who] op [perm]+
#   who     ::= [ugoa]+
#   op      ::= '+' | '-' | '='
#   perm    ::= [rwxXst]+
#
# Parameters:
#   mode_str     - The symbolic mode string
#   current_mode - The file's current permission bits (integer)
#   is_directory - Whether the target is a directory (affects 'X')
#
# Returns: The new mode as an integer.

def chmod_parse_symbolic(mode_str, current_mode, is_directory: false)
  new_mode = current_mode

  # Split on commas to get individual clauses
  clauses = mode_str.split(",")

  clauses.each do |clause|
    # Parse the "who" portion: characters before +/-/=
    match = clause.match(/\A([ugoa]*)([+\-=])([rwxXst]*)\z/)
    next unless match

    who_str = match[1]
    operator = match[2]
    perm_str = match[3]

    # Default "who" is "a" (all) if not specified
    who_str = "a" if who_str.empty?

    # Convert "who" to bit masks
    # user bits are at positions 8-6, group at 5-3, other at 2-0
    who_masks = []
    who_masks << :user if who_str.include?("u") || who_str.include?("a")
    who_masks << :group if who_str.include?("g") || who_str.include?("a")
    who_masks << :other if who_str.include?("o") || who_str.include?("a")

    # Build the permission bits to apply
    perm_bits = 0
    perm_bits |= 4 if perm_str.include?("r")
    perm_bits |= 2 if perm_str.include?("w")
    perm_bits |= 1 if perm_str.include?("x")

    # X: execute only if target is a directory or already has execute
    if perm_str.include?("X")
      if is_directory || (current_mode & 0o111) != 0
        perm_bits |= 1
      end
    end

    # Handle special bits (s, t)
    special_bits = 0
    if perm_str.include?("s")
      special_bits |= 0o4000 if who_masks.include?(:user)
      special_bits |= 0o2000 if who_masks.include?(:group)
    end
    special_bits |= 0o1000 if perm_str.include?("t")

    # Apply the operation for each who
    who_masks.each do |who|
      shift = case who
              when :user  then 6
              when :group then 3
              when :other then 0
              end

      shifted_bits = perm_bits << shift

      case operator
      when "+"
        new_mode |= shifted_bits
        new_mode |= special_bits
      when "-"
        new_mode &= ~shifted_bits
        new_mode &= ~special_bits
      when "="
        # Clear the who's bits, then set them
        mask = 7 << shift
        new_mode &= ~mask
        new_mode |= shifted_bits
        # For =, also handle special bits
        if perm_str.include?("s") || perm_str.include?("t")
          new_mode |= special_bits
        end
      end
    end
  end

  new_mode
end

# ---------------------------------------------------------------------------
# Business Logic: chmod_parse_mode
# ---------------------------------------------------------------------------
# Parse a mode string -- either octal or symbolic.
#
# Parameters:
#   mode_str     - The mode string from the command line
#   current_mode - The current file permissions (needed for symbolic modes)
#   is_directory - Whether the file is a directory
#
# Returns: [new_mode, success?]

def chmod_parse_mode(mode_str, current_mode = 0, is_directory: false)
  mode_int, is_octal = chmod_parse_octal(mode_str)
  if is_octal
    [mode_int, true]
  elsif mode_str.match?(/\A[ugoa]*[+\-=][rwxXst]+(?:,[ugoa]*[+\-=][rwxXst]+)*\z/)
    [chmod_parse_symbolic(mode_str, current_mode, is_directory: is_directory), true]
  else
    [nil, false]
  end
end

# ---------------------------------------------------------------------------
# Business Logic: chmod_apply
# ---------------------------------------------------------------------------
# Apply a mode change to a single file.
#
# Parameters:
#   path     - Path to the file
#   mode_str - The mode string (octal or symbolic)
#   opts     - Hash with :verbose, :changes, :silent
#
# Returns: [message_or_nil, success?]

def chmod_apply(path, mode_str, opts = {})
  unless File.exist?(path) || File.symlink?(path)
    return [nil, false] if opts[:silent]
    return ["chmod: cannot access '#{path}': No such file or directory", false]
  end

  old_mode = File.stat(path).mode & 0o7777
  is_dir = File.directory?(path)

  new_mode, valid = chmod_parse_mode(mode_str, old_mode, is_directory: is_dir)
  unless valid
    return ["chmod: invalid mode: '#{mode_str}'", false]
  end

  begin
    File.chmod(new_mode, path)
  rescue Errno::EPERM, Errno::EACCES => e
    return [nil, false] if opts[:silent]
    return ["chmod: changing permissions of '#{path}': #{e.message}", false]
  end

  # Build verbose/changes message
  message = nil
  if opts[:verbose]
    message = "mode of '#{path}' changed from #{format("%04o", old_mode)} to #{format("%04o", new_mode)}"
  elsif opts[:changes] && old_mode != new_mode
    message = "mode of '#{path}' changed from #{format("%04o", old_mode)} to #{format("%04o", new_mode)}"
  end

  [message, true]
end

# ---------------------------------------------------------------------------
# Business Logic: chmod_recursive
# ---------------------------------------------------------------------------
# Apply a mode change recursively to a directory tree.
#
# Parameters:
#   path     - Root directory path
#   mode_str - The mode string
#   opts     - Options hash
#
# Returns: [messages, overall_success?]

def chmod_recursive(path, mode_str, opts = {})
  messages = []
  success = true

  # Apply to the root path first
  msg, ok = chmod_apply(path, mode_str, opts)
  messages << msg if msg
  success = false unless ok

  if File.directory?(path)
    Dir.glob(File.join(path, "**", "*"), File::FNM_DOTMATCH).sort.each do |child|
      next if child.end_with?("/.", "/..")
      msg, ok = chmod_apply(child, mode_str, opts)
      messages << msg if msg
      success = false unless ok
    end
  end

  [messages, success]
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def chmod_main
  begin
    result = CodingAdventures::CliBuilder::Parser.new(CHMOD_SPEC_FILE, ["chmod"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "chmod: #{err.message}" }
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
    mode_str = result.arguments["mode"]
    files = result.arguments.fetch("files", [])
    files = [files] if files.is_a?(String)

    # If --reference is set, use that file's mode
    if result.flags["reference"]
      ref_path = result.flags["reference"]
      unless File.exist?(ref_path)
        warn "chmod: cannot stat '#{ref_path}': No such file or directory"
        exit 1
      end
      mode_str = format("%04o", File.stat(ref_path).mode & 0o7777)
    end

    opts = {
      verbose: result.flags["verbose"] || false,
      changes: result.flags["changes"] || false,
      silent: result.flags["silent"] || false,
    }

    exit_code = 0
    files.each do |file|
      if result.flags["recursive"]
        msgs, ok = chmod_recursive(file, mode_str, opts)
        msgs.each { |m| puts m }
        exit_code = 1 unless ok
      else
        msg, ok = chmod_apply(file, mode_str, opts)
        puts msg if msg
        exit_code = 1 unless ok
      end
    end

    exit exit_code
  end
end

chmod_main if __FILE__ == $PROGRAM_NAME
