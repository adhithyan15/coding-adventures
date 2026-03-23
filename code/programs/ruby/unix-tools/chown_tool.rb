#!/usr/bin/env ruby
# frozen_string_literal: true

# chown_tool.rb -- Change file owner and group
# ===============================================
#
# === What This Program Does ===
#
# This is a reimplementation of the POSIX `chown` utility. It changes
# the user and/or group ownership of files and directories.
#
# === How chown Works ===
#
#     $ chown alice file.txt           # change owner to alice
#     $ chown alice:staff file.txt     # change owner and group
#     $ chown :staff file.txt          # change group only
#     $ chown alice: file.txt          # change owner, set group to alice's login group
#     $ chown -R alice directory/      # recursive ownership change
#
# === Owner:Group Parsing ===
#
# The OWNER[:GROUP] argument supports several forms:
#
#   OWNER         - Change owner only, leave group unchanged
#   OWNER:GROUP   - Change both owner and group
#   OWNER:        - Change owner; set group to owner's login group
#   :GROUP        - Change group only, leave owner unchanged
#   OWNER.GROUP   - Same as OWNER:GROUP (historical syntax)
#
# === Permission Requirements ===
#
# Changing file ownership typically requires root (superuser) privileges.
# On most systems, only root can change the owner of a file. Any user
# can change the group to one of their supplementary groups.
#
# Our implementation handles permission errors gracefully -- the tool
# reports the error and continues with remaining files.

require "etc"
require "fileutils"
require "coding_adventures_cli_builder"

CHOWN_SPEC_FILE = File.join(File.dirname(__FILE__), "chown.json")

# ---------------------------------------------------------------------------
# Business Logic: chown_parse_owner_group
# ---------------------------------------------------------------------------
# Parse the OWNER[:GROUP] string into separate owner and group components.
#
# Supports these forms:
#   "alice"        => { owner: "alice", group: nil }
#   "alice:staff"  => { owner: "alice", group: "staff" }
#   "alice:"       => { owner: "alice", group: :login_group }
#   ":staff"       => { owner: nil,     group: "staff" }
#   "alice.staff"  => { owner: "alice", group: "staff" }
#
# Returns: Hash with :owner and :group keys.
#   :owner is a string (username or uid) or nil
#   :group is a string (groupname or gid), :login_group, or nil

def chown_parse_owner_group(spec)
  # Detect the separator: colon takes priority, then dot
  separator = if spec.include?(":")
                ":"
              elsif spec.include?(".")
                "."
              end

  if separator
    parts = spec.split(separator, 2)
    owner = parts[0].empty? ? nil : parts[0]
    group = if parts[1].nil? || parts[1].empty?
              # "alice:" form => use login group
              owner ? :login_group : nil
            else
              parts[1]
            end
    { owner: owner, group: group }
  else
    # No separator => owner only
    { owner: spec, group: nil }
  end
end

# ---------------------------------------------------------------------------
# Business Logic: chown_resolve_uid
# ---------------------------------------------------------------------------
# Resolve an owner string to a numeric UID.
#
# The owner can be a username (looked up via Etc.getpwnam) or a numeric
# UID string.
#
# Returns: [uid, error_message_or_nil]

def chown_resolve_uid(owner_str)
  return [nil, nil] unless owner_str

  # Try numeric UID first
  if owner_str.match?(/\A\d+\z/)
    return [owner_str.to_i, nil]
  end

  # Look up by name
  begin
    pw = Etc.getpwnam(owner_str)
    [pw.uid, nil]
  rescue ArgumentError
    [nil, "chown: invalid user: '#{owner_str}'"]
  end
end

# ---------------------------------------------------------------------------
# Business Logic: chown_resolve_gid
# ---------------------------------------------------------------------------
# Resolve a group string to a numeric GID. Similar to chown_resolve_uid
# but for groups.
#
# Returns: [gid, error_message_or_nil]

def chown_resolve_gid(group_str)
  return [nil, nil] unless group_str

  if group_str == :login_group
    # :login_group is handled by the caller -- we can't resolve it
    # without knowing the owner
    return [nil, nil]
  end

  if group_str.match?(/\A\d+\z/)
    return [group_str.to_i, nil]
  end

  begin
    gr = Etc.getgrnam(group_str)
    [gr.gid, nil]
  rescue ArgumentError
    [nil, "chown: invalid group: '#{group_str}'"]
  end
end

# ---------------------------------------------------------------------------
# Business Logic: chown_apply
# ---------------------------------------------------------------------------
# Apply ownership change to a single file.
#
# Parameters:
#   path     - Path to the file
#   uid      - New UID (integer or nil to leave unchanged)
#   gid      - New GID (integer or nil to leave unchanged)
#   opts     - Options hash:
#     :verbose       - Print diagnostic for every file
#     :changes       - Print diagnostic only when a change is made
#     :silent        - Suppress error messages
#     :no_dereference - Change symlink itself, not its target
#
# Returns: [message_or_nil, success?]

def chown_apply(path, uid, gid, opts = {})
  unless File.exist?(path) || File.symlink?(path)
    return [nil, false] if opts[:silent]
    return ["chown: cannot access '#{path}': No such file or directory", false]
  end

  # Get current ownership for comparison
  stat = if opts[:no_dereference] && File.symlink?(path)
           File.lstat(path)
         else
           File.stat(path)
         end
  old_uid = stat.uid
  old_gid = stat.gid

  # Determine effective uid/gid (-1 means "don't change")
  effective_uid = uid || -1
  effective_gid = gid || -1

  begin
    if opts[:no_dereference] && File.symlink?(path)
      File.lchown(effective_uid, effective_gid, path)
    else
      File.chown(effective_uid, effective_gid, path)
    end
  rescue Errno::EPERM, Errno::EACCES => e
    return [nil, false] if opts[:silent]
    return ["chown: changing ownership of '#{path}': #{e.message}", false]
  end

  # Build verbose/changes message
  message = nil
  changed = (uid && uid != old_uid) || (gid && gid != old_gid)

  if opts[:verbose]
    message = "ownership of '#{path}' retained as #{old_uid}:#{old_gid}" unless changed
    message = "changed ownership of '#{path}' from #{old_uid}:#{old_gid} to #{uid || old_uid}:#{gid || old_gid}" if changed
  elsif opts[:changes] && changed
    message = "changed ownership of '#{path}' from #{old_uid}:#{old_gid} to #{uid || old_uid}:#{gid || old_gid}"
  end

  [message, true]
end

# ---------------------------------------------------------------------------
# Business Logic: chown_recursive
# ---------------------------------------------------------------------------
# Apply ownership change recursively.
#
# Parameters:
#   path - Root directory
#   uid  - New UID (or nil)
#   gid  - New GID (or nil)
#   opts - Options hash
#
# Returns: [messages, overall_success?]

def chown_recursive(path, uid, gid, opts = {})
  messages = []
  success = true

  msg, ok = chown_apply(path, uid, gid, opts)
  messages << msg if msg
  success = false unless ok

  if File.directory?(path)
    Dir.glob(File.join(path, "**", "*"), File::FNM_DOTMATCH).sort.each do |child|
      next if child.end_with?("/.", "/..")
      msg, ok = chown_apply(child, uid, gid, opts)
      messages << msg if msg
      success = false unless ok
    end
  end

  [messages, success]
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def chown_main
  begin
    result = CodingAdventures::CliBuilder::Parser.new(CHOWN_SPEC_FILE, ["chown"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "chown: #{err.message}" }
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
    owner_group_str = result.arguments["owner_group"]
    files = result.arguments.fetch("files", [])
    files = [files] if files.is_a?(String)

    opts = {
      verbose: result.flags["verbose"] || false,
      changes: result.flags["changes"] || false,
      silent: result.flags["silent"] || false,
      no_dereference: result.flags["no_dereference"] || false,
    }

    # If --reference is set, use that file's ownership
    if result.flags["reference"]
      ref_stat = File.stat(result.flags["reference"])
      uid = ref_stat.uid
      gid = ref_stat.gid
    else
      parsed = chown_parse_owner_group(owner_group_str)

      uid, uid_err = chown_resolve_uid(parsed[:owner])
      if uid_err
        warn uid_err
        exit 1
      end

      if parsed[:group] == :login_group && uid
        # Look up the owner's login group
        begin
          pw = Etc.getpwuid(uid)
          gid = pw.gid
        rescue ArgumentError
          gid = nil
        end
      else
        gid, gid_err = chown_resolve_gid(parsed[:group])
        if gid_err
          warn gid_err
          exit 1
        end
      end
    end

    exit_code = 0
    files.each do |file|
      if result.flags["recursive"]
        msgs, ok = chown_recursive(file, uid, gid, opts)
        msgs.each { |m| puts m }
        exit_code = 1 unless ok
      else
        msg, ok = chown_apply(file, uid, gid, opts)
        puts msg if msg
        exit_code = 1 unless ok
      end
    end

    exit exit_code
  end
end

chown_main if __FILE__ == $PROGRAM_NAME
