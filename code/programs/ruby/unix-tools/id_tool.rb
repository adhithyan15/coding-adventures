#!/usr/bin/env ruby
# frozen_string_literal: true

# id_tool.rb -- Print real and effective user and group IDs
# ===========================================================
#
# === What This Program Does ===
#
# This is a reimplementation of the GNU `id` utility. It prints
# user and group information for the current user (or a specified user).
#
# === Default Output ===
#
# With no flags, id prints a comprehensive line:
#
#     uid=501(alice) gid=20(staff) groups=20(staff),501(admin),80(wheel)
#
# This shows the effective user ID, primary group ID, and all
# supplementary group memberships.
#
# === Flags ===
#
#   -u    Print only the effective user ID
#   -g    Print only the effective group ID
#   -G    Print all group IDs
#   -n    Print names instead of numbers (requires -u, -g, or -G)
#   -r    Print real ID instead of effective ID (with -u, -g, or -G)
#   -z    Separate entries with NUL instead of whitespace
#
# === How We Get User Information ===
#
# Ruby's `Etc` module wraps the POSIX passwd/group database:
#
#   Etc.getpwnam(name)  → Struct with uid, gid, name, etc.
#   Etc.getpwuid(uid)   → Same, looked up by numeric UID
#   Etc.getgrgid(gid)   → Group struct with name, gid, members
#   Process.uid         → Real user ID
#   Process.euid        → Effective user ID
#   Process.gid         → Real group ID
#   Process.egid        → Effective group ID
#   Process.groups      → Array of supplementary group IDs

require "etc"
require "coding_adventures_cli_builder"

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

ID_SPEC_FILE = File.join(File.dirname(__FILE__), "id.json")

# ---------------------------------------------------------------------------
# Business Logic: get_user_info
# ---------------------------------------------------------------------------
# Get user information for the specified user or current user.
#
# Parameters:
#   username - Username string, or nil for current user
#
# Returns: A hash with :uid, :gid, :username, :groupname, :groups
#          where :groups is an array of [gid, name] pairs.

def get_user_info(username)
  if username
    begin
      pw = Etc.getpwnam(username)
    rescue ArgumentError
      warn "id: '#{username}': no such user"
      return nil
    end
    uid = pw.uid
    gid = pw.gid
    name = pw.name
  else
    uid = Process.euid
    gid = Process.egid
    begin
      pw = Etc.getpwuid(uid)
      name = pw.name
    rescue ArgumentError
      name = uid.to_s
    end
  end

  # Look up primary group name
  groupname = begin
    Etc.getgrgid(gid).name
  rescue ArgumentError
    gid.to_s
  end

  # Get all group memberships
  group_list = if username
                 # For a specified user, scan all groups for membership
                 collect_user_groups(username, gid)
               else
                 # For current user, use Process.groups
                 Process.groups.uniq.map do |g|
                   gname = begin
                     Etc.getgrgid(g).name
                   rescue ArgumentError
                     g.to_s
                   end
                   [g, gname]
                 end
               end

  {
    uid: uid,
    gid: gid,
    username: name,
    groupname: groupname,
    groups: group_list
  }
end

# ---------------------------------------------------------------------------
# Business Logic: get_real_user_info
# ---------------------------------------------------------------------------
# Get real (not effective) user/group IDs.

def get_real_ids
  {
    uid: Process.uid,
    gid: Process.gid
  }
end

# ---------------------------------------------------------------------------
# Business Logic: collect_user_groups
# ---------------------------------------------------------------------------
# Collect all group memberships for a given username by scanning
# the group database.
#
# Parameters:
#   username     - The username to look up
#   primary_gid  - The user's primary group ID
#
# Returns: Array of [gid, name] pairs.

def collect_user_groups(username, primary_gid)
  groups = []
  # Always include primary group
  primary_name = begin
    Etc.getgrgid(primary_gid).name
  rescue ArgumentError
    primary_gid.to_s
  end
  groups << [primary_gid, primary_name]

  # Scan all groups for additional membership
  Etc.group do |gr|
    if gr.mem.include?(username) && gr.gid != primary_gid
      groups << [gr.gid, gr.name]
    end
  end

  groups
end

# ---------------------------------------------------------------------------
# Business Logic: format_id_default
# ---------------------------------------------------------------------------
# Format the default id output: uid=N(name) gid=N(name) groups=N(name),...

def format_id_default(info)
  parts = []
  parts << "uid=#{info[:uid]}(#{info[:username]})"
  parts << "gid=#{info[:gid]}(#{info[:groupname]})"

  group_strs = info[:groups].map { |gid, name| "#{gid}(#{name})" }
  parts << "groups=#{group_strs.join(",")}"

  parts.join(" ")
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def id_main
  # --- Step 1: Parse arguments ---------------------------------------------
  begin
    result = CodingAdventures::CliBuilder::Parser.new(ID_SPEC_FILE, ["id"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "id: #{err.message}" }
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
    username = result.arguments["user_name"]
    info = get_user_info(username)
    exit 1 unless info

    show_name = result.flags["name"]
    show_real = result.flags["real"]
    separator = result.flags["zero"] ? "\0" : "\n"

    if result.flags["user"]
      # -u: print user ID
      if show_real && !username
        uid = Process.uid
        if show_name
          print Etc.getpwuid(uid).name
        else
          print uid
        end
      else
        if show_name
          print info[:username]
        else
          print info[:uid]
        end
      end
      print separator
    elsif result.flags["group"]
      # -g: print group ID
      if show_real && !username
        gid_val = Process.gid
        if show_name
          print Etc.getgrgid(gid_val).name
        else
          print gid_val
        end
      else
        if show_name
          print info[:groupname]
        else
          print info[:gid]
        end
      end
      print separator
    elsif result.flags["groups"]
      # -G: print all group IDs
      group_sep = result.flags["zero"] ? "\0" : " "
      values = info[:groups].map do |gid, name|
        show_name ? name : gid.to_s
      end
      print values.join(group_sep)
      print separator
    else
      # Default: full output
      print format_id_default(info)
      print separator
    end
  end
end

# Only run main when this file is executed directly.
id_main if __FILE__ == $PROGRAM_NAME
